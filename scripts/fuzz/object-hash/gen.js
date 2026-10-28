//! Builds the object-hash fuzz corpus: random values, each with the reference's
//! canonical byte stream and digest. Run with the `object-hash` npm package on
//! the module path. See scripts/fuzz/README.md.

const objectHash = require('object-hash');

const NUMS = [
  0, 1, -1, 42, 0.5, 1e21, 1e-7, 1e20, 1 / 3, 9007199254740992, -0.0,
  NaN, Infinity, -Infinity, 123.456, 1e-6, 5e-324, 1.7976931348623157e308,
  1.23e22, -1e21, 1e100,
];
const STRS = ['', 'a', 'ab', 'café', '中文', '😀', 'a:b', '42', '\u0000x', '\uFFFFz'];

// A seeded PRNG, so the corpus is the same every run. Math.random would make it
// impossible to rebuild a fixture from the committed script.
let state = 0x2f6e2b1;
function random() {
  // xorshift32
  state ^= state << 13;
  state ^= state >>> 17;
  state ^= state << 5;
  state >>>= 0;
  return state / 0x100000000;
}

function pick(items) {
  return items[Math.floor(random() * items.length)];
}

function randomValue(depth) {
  if (depth <= 0 || random() < 0.35) {
    const kind = Math.floor(random() * 4);
    if (kind === 0) return null;
    if (kind === 1) return random() < 0.5;
    if (kind === 2) return pick(NUMS);
    return pick(STRS);
  }
  const kind = Math.floor(random() * 4);
  const length = Math.floor(random() * 4);
  const keys = ['a', 'b', 'café', '', 'k1', 'z'];
  if (kind === 0) {
    return Array.from({ length }, () => randomValue(depth - 1));
  }
  if (kind === 1) {
    const set = new Set();
    for (let i = 0; i < length; i++) set.add(randomValue(depth - 1));
    return set;
  }
  if (kind === 2) {
    const map = new Map();
    for (let i = 0; i < length; i++) map.set(pick(keys), randomValue(depth - 1));
    return map;
  }
  const object = {};
  for (let i = 0; i < length; i++) object[pick(keys)] = randomValue(depth - 1);
  return object;
}

// The reference writes its canonical form to a stream. A passthrough algorithm
// hands back exactly the bytes the digest is taken over.
function canonical(value, options) {
  let bytes = '';
  const stream = { write: (x) => (bytes += x), update: (x) => (bytes += x) };
  objectHash.writeToStream(value, { algorithm: 'passthrough', ...options }, stream);
  return bytes;
}

// The value as a tagged string the Rust side can rebuild: `{"n":..}` numbers,
// `{"a":..}` arrays, `{"s":..}` sets, `{"m":..}` maps, `{"o":..}` plain objects.
// Numbers go through `String` so NaN, the infinities and the exponent forms
// survive. `respectType:false` is passed when a plain object is hashed, because
// the reference's default adds a Node-runtime prototype preamble this crate
// cannot reproduce; see the crate documentation.
function tagged(value) {
  if (value === null) return 'null';
  if (typeof value === 'boolean') return value ? 'true' : 'false';
  if (typeof value === 'number') {
    if (Number.isNaN(value)) return '{"n":"NaN"}';
    if (value === Infinity) return '{"n":"Infinity"}';
    if (value === -Infinity) return '{"n":"-Infinity"}';
    return '{"n":"' + String(value) + '"}';
  }
  if (typeof value === 'string') return JSON.stringify(value);
  if (Array.isArray(value)) return '{"a":[' + value.map(tagged).join(',') + ']}';
  if (value instanceof Set) return '{"s":[' + [...value].map(tagged).join(',') + ']}';
  if (value instanceof Map) {
    const parts = [];
    for (const [key, item] of value) parts.push('[' + JSON.stringify(key) + ',' + tagged(item) + ']');
    return '{"m":[' + parts.join(',') + ']}';
  }
  const parts = [];
  for (const key of Object.keys(value)) parts.push('[' + JSON.stringify(key) + ',' + tagged(value[key]) + ']');
  return '{"o":[' + parts.join(',') + ']}';
}

// A plain object has to be hashed with `respectType:false` to be reproducible
// outside JavaScript, and the same option has to be passed when it is nested.
function optionsFor(value) {
  let needs = false;
  (function walk(v) {
    if (v === null || typeof v !== 'object') return;
    if (Array.isArray(v) || v instanceof Set || v instanceof Map) {
      for (const item of v instanceof Map ? v.values() : v) walk(item);
      return;
    }
    needs = true;
    for (const key of Object.keys(v)) walk(v[key]);
  })(value);
  return needs ? { respectType: false } : {};
}

const rows = [];
for (let i = 0; i < 4000; i++) {
  const value = randomValue(3);
  const options = optionsFor(value);
  rows.push(tagged(value) + '\t' + canonical(value, options) + '\t' + objectHash(value, options));
}

// A second corpus of plain objects, since the first one reaches them only by
// chance. Each row carries the reference's `excludeValues` and `unorderedArrays`
// too, so the options are pinned and not only the default path.
const objectRows = [];
const KEYS = ['a', 'b', 'café', '', 'k1', 'z', 'prototype'];
const SCALARS = [null, true, false, 0, 1, -1, 0.5, 1e21, 'x', '', '中文', '😀'];
function randomObject(depth) {
  const object = {};
  const count = Math.floor(random() * 5);
  for (let i = 0; i < count; i++) {
    object[pick(KEYS)] =
      depth > 0 && random() < 0.5 ? randomObject(depth - 1) : pick(SCALARS);
  }
  return object;
}
for (let i = 0; i < 2000; i++) {
  const value = randomObject(2);
  const excludeValues = random() < 0.25;
  const unorderedArrays = random() < 0.25;
  const options = { respectType: false, excludeValues, unorderedArrays };
  const flags = (excludeValues ? '1' : '0') + (unorderedArrays ? '1' : '0');
  objectRows.push(
    tagged(value) + '\t' + flags + '\t' + canonical(value, options) + '\t' + objectHash(value, options),
  );
}

// The curated oracle file also carries SHA-256 and MD5, so the Rust side can be
// held to all three algorithms the reference offers, not just its default.
//
// A set of curated plain-object cases is appended first. They are hashed with
// `respectType:false`, the only form reproducible outside JavaScript, so the
// `object:` encoding is pinned by name and not only by the random corpus.
const OBJECT_CASES = {
  'empty-object': {},
  'object-one': { a: 1 },
  'object-two': { a: 1, b: 2 },
  'object-reversed': { b: 2, a: 1 },
  'object-nested': { a: { b: 1 } },
  'object-unicode-key': { café: 'x' },
  'object-empty-string-key': { '': 1 },
  'object-in-array': [{ a: 1 }, { b: 2 }],
  'object-array-value': { list: [1, 2, 3] },
  'object-null-value': { a: null },
};

function appendObjectOracle(path) {
  const crypto = require('crypto');
  const lines = [];
  for (const [name, value] of Object.entries(OBJECT_CASES)) {
    const text = canonical(value, { respectType: false });
    const buf = Buffer.from(text, 'utf8');
    lines.push(
      [
        name,
        text,
        crypto.createHash('sha1').update(buf).digest('hex'),
        crypto.createHash('sha256').update(buf).digest('hex'),
        crypto.createHash('md5').update(buf).digest('hex'),
      ].join('\t'),
    );
  }
  require('fs').appendFileSync(path, lines.join('\n') + '\n');
  console.log('appended', lines.length, 'object cases to', path);
}

function extended(path) {
  const crypto = require('crypto');
  const lines = require('fs').readFileSync(path, 'utf8').split('\n').filter((l) => l.length);
  const out = [lines[0] + '\tsha256\tmd5'];
  for (const line of lines.slice(1)) {
    const canonical = line.split('\t')[1];
    const buf = Buffer.from(canonical, 'utf8');
    out.push(
      line +
        '\t' +
        crypto.createHash('sha256').update(buf).digest('hex') +
        '\t' +
        crypto.createHash('md5').update(buf).digest('hex'),
    );
  }
  require('fs').writeFileSync(path, out.join('\n') + '\n');
  console.log('extended', out.length - 1, 'rows in', path);
}

// Adds the SHA-512 digest and the base64 of the SHA-1 digest to each row, so
// the Rust side is held to the fourth algorithm the reference offers and to its
// non-hexadecimal encoding.
function extended512(path) {
  const crypto = require('crypto');
  const lines = require('fs').readFileSync(path, 'utf8').split('\n').filter((l) => l.length);
  const out = [lines[0] + '\tsha512\tbase64'];
  for (const line of lines.slice(1)) {
    const canonical = line.split('\t')[1];
    const buf = Buffer.from(canonical, 'utf8');
    out.push(
      line +
        '\t' +
        crypto.createHash('sha512').update(buf).digest('hex') +
        '\t' +
        crypto.createHash('sha1').update(buf).digest('base64'),
    );
  }
  require('fs').writeFileSync(path, out.join('\n') + '\n');
  console.log('extended512', out.length - 1, 'rows in', path);
}

// Adds the SHA-384 digest to each row, so the Rust side is held to the fifth
// algorithm the reference offers. SHA-384 is SHA-512 with a different initial
// state and a truncated output, so it is the one most likely to be transcribed
// wrong and worth pinning against the reference rather than only against the
// FIPS vectors.
function extended384(path) {
  const crypto = require('crypto');
  const lines = require('fs').readFileSync(path, 'utf8').split('\n').filter((l) => l.length);
  const out = [lines[0] + '\tsha384'];
  for (const line of lines.slice(1)) {
    const canonical = line.split('\t')[1];
    const buf = Buffer.from(canonical, 'utf8');
    out.push(line + '\t' + crypto.createHash('sha384').update(buf).digest('hex'));
  }
  require('fs').writeFileSync(path, out.join('\n') + '\n');
  console.log('extended384', out.length - 1, 'rows in', path);
}

// Adds the SHA-224 digest to each row, so the Rust side is held to the sixth
// algorithm the reference offers. SHA-224 is SHA-256 with a different initial
// state and a truncated output, so like SHA-384 it is worth pinning against the
// reference rather than only against the FIPS vectors.
function extended224(path) {
  const crypto = require('crypto');
  const lines = require('fs').readFileSync(path, 'utf8').split('\n').filter((l) => l.length);
  const out = [lines[0] + '\tsha224'];
  for (const line of lines.slice(1)) {
    const canonical = line.split('\t')[1];
    const buf = Buffer.from(canonical, 'utf8');
    out.push(line + '\t' + crypto.createHash('sha224').update(buf).digest('hex'));
  }
  require('fs').writeFileSync(path, out.join('\n') + '\n');
  console.log('extended224', out.length - 1, 'rows in', path);
}

const target = process.argv[2];
if (!target) {
  console.error('usage: node gen.js <fuzz.tsv> <objects.tsv> [oracle.tsv to extend] [oracle.tsv to append object cases to]');
  process.exit(1);
}
require('fs').writeFileSync(target, rows.join('\n') + '\n');
console.log('wrote', rows.length, 'cases to', target);
const objectTarget = process.argv[3];
if (objectTarget) {
  require('fs').writeFileSync(objectTarget, objectRows.join('\n') + '\n');
  console.log('wrote', objectRows.length, 'cases to', objectTarget);
}
if (process.argv[4]) extended(process.argv[4]);
if (process.argv[5]) appendObjectOracle(process.argv[5]);
if (process.argv[6]) extended512(process.argv[6]);
if (process.argv[7]) extended384(process.argv[7]);
if (process.argv[8]) extended224(process.argv[8]);
