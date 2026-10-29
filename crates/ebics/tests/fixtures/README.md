# The ebics oracle fixtures

None of these was produced by this crate. Each was made with a reference
implementation, and each is rebuilt by a committed generator, so none is a value
nobody can reproduce.

| file | what it is | generator |
| --- | --- | --- |
| `rsa_2048.json` | a 2048-bit key and the values Python computed with it: the key digest, a PKCS#1 v1.5 and a PSS signature, and a wrapped key | `scripts/fuzz/ebics/gen.py` |
| `bank_response.xml` | a complete EBICS response, with a real RSA wrap and a real AES-128-CBC encryption of deflated order data | `scripts/fuzz/ebics/gen.py` |
| `bank_order_data.bin` | the order data `bank_response.xml` decrypts to | input to the generator, not an output |
| `hpb_request.xml` | a real HPB request from the npm `ebics-client`, carrying the digest that client computed over its own header | `scripts/fuzz/ebics/gen.js` |
| `hpb_pubkey.txt` | the base64 modulus and exponent of the key that signed `hpb_request.xml` | `scripts/fuzz/ebics/gen.js` |

The RSA and response fixtures come from Python's `cryptography` and `zlib`. The
key is committed as its prime factors (`scripts/fuzz/ebics/rsa_key.json`) so it
rebuilds without the randomness of key generation. `rsa_2048.json` is stable
apart from the PSS signature and the ciphertext, which PKCS#1 padding randomises;
`bank_response.xml` is likewise not byte-stable. The tests assert on the
recovered key and the decrypted order data, never on those bytes.

`hpb_request.xml` is the one worth understanding. Its `ds:DigestValue` is the
check that matters most: `crates/ebics/tests/c14n_oracle.rs` reproduces it from
the message alone, which exercises the parse, the exclusive-C14N, the namespace
injection and SHA-256 together against an independent implementation. The key
that made its `AuthSignature` is drawn fresh by the reference each run, so the
signature was not checkable when the fixture was first committed; the generator
now pins the key and writes its public half to `hpb_pubkey.txt`, and the crate
verifies the signature, which is a second, end-to-end check on the same bytes.
