#!/usr/bin/env python3
"""Rebuilds the `ebics` fixtures that a Python reference produced.

Three of the four fixtures under `crates/ebics/tests/fixtures` were made with
Python's `cryptography` and `zlib`, and were committed without a generator, so
they could not be rebuilt. This is that generator. The fourth, `hpb_request.xml`,
comes from the JavaScript `ebics-client` package and is rebuilt by `gen.js`
beside this file.

Outputs:

    rsa_2048.json        a 2048-bit key and values Python computed with it
    bank_response.xml    an EBICS response Python built, wrapping a key and
                         encrypting order data
    bank_order_data.bin  the order data that response decrypts to

Two keys are involved, each committed beside this script. `rsa_2048.json` uses
the 2048-bit `rsa_key.json`; the response wraps its AES key under the 1024-bit
`rsa_key_1024.json`, the same key the Rust tests hold as `MODULUS`/`PRIVATE`. A
key is committed as its prime factors so it rebuilds without the randomness of
`generate_key_pair`.

`rsa_2048.json` is reproducible exactly, apart from the PSS signature and the
RSA ciphertext, which PKCS#1 padding randomises. `bank_response.xml` is likewise
not byte-stable. The tests assert on the decrypted result and the recovered key,
not on the bytes, so a rebuild keeps passing.

Usage:
    pip install cryptography
    python3 scripts/fuzz/ebics/gen.py [fixture-dir]

The fixture directory defaults to `crates/ebics/tests/fixtures`.
"""

import base64
import json
import pathlib
import re
import sys
import zlib

from cryptography.hazmat.primitives import hashes
from cryptography.hazmat.primitives.asymmetric import padding, rsa
from cryptography.hazmat.primitives.ciphers import Cipher, algorithms, modes

ROOT = pathlib.Path(__file__).resolve().parents[3]
DEFAULT_DIR = ROOT / "crates" / "ebics" / "tests" / "fixtures"

# The message the signature fixture is over, and the plaintext the RSA-encrypt
# fixture wraps. Fixed so the fixtures rebuild to the same shape.
SIGNATURE_MESSAGE = b"EBICS order data digest fixture"
WRAPPED_PLAINTEXT = b"0123456789abcdef"

# The AES key the response was encrypted under, and the IV. EBICS uses a zero
# IV; the key here is a fixed one so the fixture is reproducible.
AES_KEY = bytes(range(16))
AES_IV = bytes(16)


def b64(data: bytes) -> str:
    return base64.b64encode(data).decode()


def load_key(key_path: pathlib.Path) -> rsa.RSAPrivateKey:
    data = json.loads(key_path.read_text())
    n = int.from_bytes(base64.b64decode(data["n"]), "big")
    e = int.from_bytes(base64.b64decode(data["e"]), "big")
    d = int.from_bytes(base64.b64decode(data["d"]), "big")
    return rsa.RSAPrivateNumbers(
        p=data["p"],
        q=data["q"],
        d=d,
        dmp1=d % (data["p"] - 1),
        dmq1=d % (data["q"] - 1),
        iqmp=pow(data["q"], -1, data["p"]),
        public_numbers=rsa.RSAPublicNumbers(e, n),
    ).private_key()


def write_key_fixture(key: rsa.RSAPrivateKey, out: pathlib.Path) -> None:
    numbers = key.private_numbers()
    public = numbers.public_numbers

    def be(value: int) -> bytes:
        return value.to_bytes((value.bit_length() + 7) // 8, "big")

    modulus = be(public.n)
    exponent = be(public.e)

    # The key digest is the EBICS one: SHA-256 over the hex exponent, a space,
    # then the hex modulus, each with leading zeroes trimmed. The crate computes
    # the same value in `PublicKey::digest_bytes`.
    text = f"{exponent.hex().lstrip('0')} {modulus.hex().lstrip('0')}"
    digest = __import__("hashlib").sha256(text.encode()).digest()

    fixture = {
        "n": b64(modulus),
        "e": b64(exponent),
        "d": b64(be(numbers.d)),
        "digest": b64(digest),
        "pkcs1v15_sig": b64(
            key.sign(SIGNATURE_MESSAGE, padding.PKCS1v15(), hashes.SHA256())
        ),
        "pss_sig": b64(
            key.sign(
                SIGNATURE_MESSAGE,
                padding.PSS(mgf=padding.MGF1(hashes.SHA256()), salt_length=32),
                hashes.SHA256(),
            )
        ),
        "ct": b64(key.public_key().encrypt(WRAPPED_PLAINTEXT, padding.PKCS1v15())),
        "pt": WRAPPED_PLAINTEXT.decode(),
    }
    out.write_text(json.dumps(fixture, indent=2) + "\n")


def ebics_pad(data: bytes) -> bytes:
    """Zero padding with the count in the last byte, as EBICS defines it."""
    padding = 16 - (len(data) % 16)
    return data + bytes(padding - 1) + bytes([padding])


def write_response_fixture(
    key: rsa.RSAPrivateKey, order_data: bytes, out: pathlib.Path
) -> None:
    compressed = zlib.compress(order_data, 9)

    encryptor = Cipher(algorithms.AES(AES_KEY), modes.CBC(AES_IV)).encryptor()
    ciphertext = encryptor.update(ebics_pad(compressed)) + encryptor.finalize()

    wrapped = key.public_key().encrypt(AES_KEY, padding.PKCS1v15())

    document = f"""<?xml version="1.0" encoding="UTF-8"?>
<ebicsKeyManagementResponse xmlns="urn:org:ebics:H004" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" Revision="1" Version="H004">
  <header authenticate="true">
    <static/>
    <mutable>
      <ReturnCode>000000</ReturnCode>
      <ReportText>[EBICS_OK] OK</ReportText>
    </mutable>
  </header>
  <body>
    <DataTransfer>
      <DataEncryptionInfo authenticate="true">
        <EncryptionPubKeyDigest Algorithm="http://www.w3.org/2001/04/xmlenc#sha256" Version="E002">{b64(b"x" * 20)}</EncryptionPubKeyDigest>
        <TransactionKey>{b64(wrapped)}</TransactionKey>
      </DataEncryptionInfo>
      <OrderData>{b64(ciphertext)}</OrderData>
    </DataTransfer>
    <ReturnCode authenticate="true">000000</ReturnCode>
    <TimestampBankParameter authenticate="true">2016-08-10T16:16:17.512Z</TimestampBankParameter>
  </body>
</ebicsKeyManagementResponse>
"""
    out.write_text(document)


def main() -> int:
    fixture_dir = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_DIR
    key_2048 = load_key(ROOT / "scripts" / "fuzz" / "ebics" / "rsa_key.json")
    key_1024 = load_key(ROOT / "scripts" / "fuzz" / "ebics" / "rsa_key_1024.json")

    write_key_fixture(key_2048, fixture_dir / "rsa_2048.json")

    order_data = (fixture_dir / "bank_order_data.bin").read_bytes()
    write_response_fixture(key_1024, order_data, fixture_dir / "bank_response.xml")

    print("rsa_2048.json rebuilt")
    print("bank_response.xml rebuilt")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
