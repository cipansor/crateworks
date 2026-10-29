#!/usr/bin/env node
'use strict';

// Rebuilds `crates/ebics/tests/fixtures/hpb_request.xml`.
//
// That fixture is a real EBICS HPB request produced by the npm `ebics-client`
// package (MIT), the reference this crate's canonicalisation and digest were
// checked against. It was committed without a generator, so it could not be
// rebuilt. This is that generator.
//
// The point of the fixture is the `ds:DigestValue`: it is what the reference
// computed over its own header, and `crates/ebics/tests/c14n_oracle.rs`
// reproduces it from the message alone. That value does not depend on the
// signing key, so any key works; the signature over it is over a key that is
// not in this repository and is not checked.
//
// The reference reads the clock and the RNG for the `Nonce` and `Timestamp`
// header fields. Both are pinned below, so the digest is stable across runs.
//
// Usage:
//     npm install   # at the repository root; ebics-client is a dev dependency
//     NODE_PATH=<repo>/node_modules node scripts/fuzz/ebics/gen.js [fixture-dir]
//
// The fixture directory defaults to `crates/ebics/tests/fixtures`.

const path = require('path');
const fs = require('fs');

const FIXTURE_DIR = process.argv[2]
  || path.join(__dirname, '..', '..', '..', 'crates', 'ebics', 'tests', 'fixtures');

// The two header fields the reference draws from outside itself. Pinned so the
// digest reproduces; the same values are in the committed fixture.
const NONCE = 'c5baa99837e84f97dc5f015131a0bb77';
const TIMESTAMP = '2026-10-22T11:44:26.627Z';

// The account the request is for. These are the values in the committed
// fixture, and they appear verbatim in the header.
const HOST_ID = 'CRSISOTB';
const PARTNER_ID = 'CRS04381';
const USER_ID = 'CRS04381';

// A fixed key, so the signature is stable across runs and `hpb_pubkey.txt` can
// name the public half the crate verifies it with. The reference draws a fresh
// key every run otherwise, which is why the signature in an early fixture could
// not be checked at all.
const SIGNING_KEY = fs.readFileSync(path.join(__dirname, 'hpb_key.pem'), 'utf8');

const Crypto = require('ebics-client/lib/crypto/Crypto');
Crypto.nonce = () => NONCE;
Crypto.timestamp = () => TIMESTAMP;

const ebics = require('ebics-client');
const Keys = require('ebics-client/lib/keymanagers/Keys');
const serializer = require('ebics-client/lib/middleware/serializer');
const signer = require('ebics-client/lib/orders/H004/signer');

const memory = {};
const keyStorage = {
  async write(data) { memory.keys = data; },
  async read() { return memory.keys; },
};

(async () => {
  const client = new ebics.Client({
    url: 'https://bank.example/ebics',
    partnerId: PARTNER_ID,
    userId: USER_ID,
    hostId: HOST_ID,
    // aes-256-cbc needs a 32-byte key; the reference uses it to encrypt its own
    // key store, which has nothing to do with the request being built.
    passphrase: '0123456789abcdef0123456789abcdef',
    keyStorage,
  });

  const keys = new Keys({ X002: SIGNING_KEY });

  const order = {
    version: 'h004',
    orderDetails: { OrderType: 'HPB', OrderAttribute: 'DZHNN' },
    operation: 'ini',
  };

  const xml = (await serializer.use(order, client)).toXML();
  const signed = signer.sign(xml, keys.x());

  const target = path.join(FIXTURE_DIR, 'hpb_request.xml');
  fs.writeFileSync(target, signed);
  console.log(`wrote ${target}`);

  // The public half, base64, one field per line, so the Rust test can rebuild
  // the key and verify the signature the reference made. `n('hex')` drops the
  // leading zero byte node-forge keeps in its signed big-endian form.
  const publicKey = keys.x();
  const modulus = Buffer.from(publicKey.n('hex'), 'hex');
  const exponent = Buffer.from(publicKey.e('hex'), 'hex');
  const pubTarget = path.join(FIXTURE_DIR, 'hpb_pubkey.txt');
  fs.writeFileSync(pubTarget, `${modulus.toString('base64')}\n${exponent.toString('base64')}\n`);
  console.log(`wrote ${pubTarget}`);
})().catch((error) => {
  console.error(error);
  process.exit(1);
});
