#!/usr/bin/env node
/*
 * Corpus generator for the RFC 7239 `Forwarded` parser in `crates/proxy-addr`.
 *
 * Drives the npm `forwarded-parse` package over a generated corpus of header
 * values and writes `crates/proxy-addr/tests/fixtures/forwarded.tsv`.
 *
 * The oracle is the reference itself, so a disagreement is a bug in the crate.
 * Two things are recorded per case:
 *
 *   OK   the elements in header order, each a sorted `name:value` list, with
 *        names and values JSON-quoted so a boundary can never be ambiguous
 *   ERR  `!pos:<n>` for an unexpected character at index n, `!eof` otherwise
 *
 * A header the reference rejects is written as `!...` so the crate is checked
 * to reject the same inputs, not merely to agree on the accepted ones. The
 * index is compared too: a parser that stops at the right character but
 * reports the wrong one is not the same parser.
 *
 * One deliberate divergence is recorded rather than hidden. RFC 7230 section 7
 * requires a recipient to ignore empty list elements, and the reference does
 * not, so a header it rejects only for `,,` is one this crate must read. Such a
 * row is marked `empty` and carries the reference's verdict for the header with
 * the empty elements removed; `tests/forwarded_oracle.rs` compares it as
 * described there.
 *
 * The header text is hex-encoded, because a header may hold a tab, a newline or
 * a NUL and a TSV field cannot.
 *
 * Needs `npm install forwarded-parse@2.2.0` in the repository root (the
 * manifest is committed). Run from the repository root:
 *
 *   node scripts/fuzz/proxy-addr/gen_forwarded.js
 */

'use strict'

const fs = require('fs')
const path = require('path')
const parse = require('forwarded-parse')

// A differential gate compares the crate against whatever the oracle is today,
// so a new release of the oracle would re-baseline the corpus silently and
// report as a crate defect. Pin it, as the other generators in this repository
// do, and let a deliberate upgrade be its own change.
const PINNED = '2.2.0'
const installed = require('forwarded-parse/package.json').version
if (installed !== PINNED) {
  console.error(
    'forwarded-parse is ' + installed + ', but this generator is pinned to ' +
    PINNED + '. Install that version (`npm install forwarded-parse@' + PINNED +
    '`) or update PINNED deliberately, re-running the corpus check afterwards.'
  )
  process.exit(1)
}

// `OUT` and `SEED` let the verification gate write a fresh corpus to a
// temporary file and leave the committed one alone.
const OUT = process.env.OUT || path.join(__dirname, '..', '..', '..', 'crates',
  'proxy-addr', 'tests', 'fixtures', 'forwarded.tsv')

// A tiny deterministic PRNG so the corpus is reproducible without a seed file.
function mulberry32(a) {
  return function () {
    a |= 0
    a = (a + 0x6D2B79F5) | 0
    let t = Math.imul(a ^ (a >>> 15), 1 | a)
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296
  }
}

const rand = mulberry32(Number(process.env.SEED || 0x9e3779b9))
const pick = (arr) => arr[Math.floor(rand() * arr.length)]

const NAMES = ['for', 'by', 'proto', 'host', 'For', 'BY', 'Proto', 'x-unknown',
  'a', 'zz', 'for', 'by']

// Values that are a legal `token` (RFC 7230 section 3.2.6) as written. A colon,
// a bracket or a non-ASCII byte is not, so those values live in QUOTED_ONLY and
// reach the parser through a quoted-string.
const TOKEN_VALUES = [
  '192.0.2.43', '198.51.100.17', '203.0.113.43', 'unknown', '_hidden', '_a-b',
  'example.com', 'https', 'http', 'x', 'a.b.c', 'a-b_c', ''
]
const QUOTED_VALUES = [
  '""', '"quoted"', '"a\\"b"', '"\\"x\\""', '"a;b"', '"a,b"', '"a b"',
  '"tab\there"', '"caf\u00e9"', '"[2001:db8::1]:4711"', '"1.2.3.4:80"',
  '"extended \u00e9\u00ff"', '"_obs"', '"\\\\backslash"'
]
const VALUES = TOKEN_VALUES.concat(QUOTED_VALUES)
const SEPARATORS = [';', ',', '; ', ', ', ' ; ', ',  ', '  ', '\t', ',\t', ' ;\t']

// The characters the reference distinguishes, plus non-ASCII: %x80-FF is
// `extended` and legal inside a quoted-string, above that it is not.
const ALPHABET = [
  'a', 'Z', '0', '9', '!', '#', "'", '*', '+', '-', '.', '^', '_', '`', '|', '~',
  '=', '"', '\\', ';', ',', ' ', '\t', '(', ')', '/', ':', '<', '>', '?', '@',
  '[', ']', '{', '}', '\u00e9', '\u00fc', '\u00ff', '\u0080', '\u65e5',
  '\ud83d\ude00', '\n', '\u0000'
]

// A header built from whole parameters, the shape a real proxy produces.
function structured () {
  const elements = 1 + Math.floor(rand() * 4)
  let out = ''
  for (let i = 0; i < elements; i++) {
    const count = 1 + Math.floor(rand() * 3)
    let element = ''
    for (let j = 0; j < count; j++) {
      if (j) element += pick(SEPARATORS)
      element += pick(NAMES) + '=' + pick(VALUES)
    }
    if (i) out += pick([',', ', ', ',  ', ',\t', ' , '])
    out += element
  }
  return out
}

// Random text over the reference's character classes, which is what exercises
// the error positions.
function noisy () {
  const length = Math.floor(rand() * 24)
  let out = ''
  for (let i = 0; i < length; i++) out += pick(ALPHABET)
  return out
}

function corrupt (header) {
  const position = Math.floor(rand() * (header.length + 1))
  const junk = pick([' ', '=', '"', '\\', ';', ',', '\t', 'x', '(', '\u00e9'])
  return header.slice(0, position) + junk + header.slice(position)
}

// Render the reference's result unambiguously. A bare `name=value` join cannot
// distinguish `a="b,c=d"` from `a=b;c=d`: both would read as the same text, so a
// parser that turned quoted text into a separate parameter would still agree.
// Quoting every name and value the way JSON does keeps the boundary visible.
function renderElements (elements) {
  const parts = elements.map((element) => {
    const pairs = Object.keys(element).sort()
      .map((key) => JSON.stringify(key) + ':' + JSON.stringify(element[key]))
    return '{' + pairs.join(',') + '}'
  })
  return 'OK[' + parts.join('|') + ']'
}

function verdict (header) {
  try {
    return renderElements(parse(header))
  } catch (error) {
    const found = /Unexpected character/.exec(error.message)
    return found ? '!pos:' + error.message.match(/index (\d+)/)[1] : '!eof'
  }
}

// Split a header on the commas that separate elements, respecting quoted
// strings, and drop the segments that hold no element.
//
// RFC 7230 section 7 requires a recipient to ignore empty list elements, and
// the reference does not, so a header the reference rejects for that reason is
// one this crate must still read. Rewriting the header without the empty
// segments gives a header the reference does accept, which is what proves the
// divergence is exactly that rule and nothing else.
function withoutEmptyElements (header) {
  const segments = []
  let current = ''
  let inQuotes = false
  let escaping = false
  for (const c of header) {
    if (escaping) {
      escaping = false
    } else if (c === '\\' && inQuotes) {
      escaping = true
    } else if (c === '"') {
      inQuotes = !inQuotes
    } else if (c === ',' && !inQuotes) {
      segments.push(current)
      current = ''
      continue
    }
    current += c
  }
  segments.push(current)
  // An empty element is a separator with nothing between it and its
  // neighbour. The first segment counts only when it is empty outright: a
  // leading tab or space is leading whitespace, which RFC 7230's list rule does
  // not allow and the crate must still reject, so trimming it away here would
  // hide a second fault behind the empty-element one.
  return segments
    .filter((segment, index) => index === 0
      ? segment !== ''
      : segment.replace(/^[ \t]+|[ \t]+$/g, '') !== '')
    .join(',')
}

// One row of the corpus: the header, how strictly it is compared, and the
// reference's verdict.
//
//   `strict`  the reference accepted or rejected the header as written, so the
//             crate must reproduce the verdict exactly, index included
//   `empty`   the header holds an empty list element, which RFC 7230 section 7
//             says a recipient must ignore and the reference does not. The
//             verdict is the reference's answer for the header with those
//             elements removed: `OK[...]` must be reproduced exactly, while a
//             bare `!` means the remainder is still invalid and the crate must
//             reject it. The index is not compared there, because skipping an
//             element shifts every later position.
function row (header) {
  const direct = verdict(header)
  if (!direct.startsWith('!')) return ['strict', direct]
  const stripped = withoutEmptyElements(header)
  if (stripped === header) return ['strict', direct]
  const repaired = verdict(stripped)
  return ['empty', repaired.startsWith('!') ? '!' : repaired]
}

// A TSV field cannot hold a tab, a newline or a NUL, so the header is
// hex-encoded as UTF-8.
function encode (header) {
  return Buffer.from(header, 'utf8').toString('hex')
}

const lines = [
  '# Generated by scripts/fuzz/proxy-addr/gen_forwarded.js from npm forwarded-parse 2.2.0.',
  '# Each row is: header (hex UTF-8), strictness, then the expected parse.',
  '# strict  the reference\'s own verdict, to be reproduced exactly.',
  '# empty   the reference rejected the header only for an empty list element;',
  '#         the verdict is its answer with those elements removed (RFC 7230 s7).',
  '# OK[elements] lists elements in header order, each a sorted name:value list',
  '# with JSON-quoted names and values, so a boundary is never ambiguous.',
  '# !pos:<n> is an unexpected character at index n; !eof is an early end.'
]
const HEADER = lines.length

// A handful of hand-written headers first, so a reader can see the format
// without decoding the generated ones. These are the cases the RFC names, plus
// the empty-element shape that the crate is deliberately more tolerant of.
const SEEDS = [
  'for=192.0.2.43',
  'for=192.0.2.43, for=198.51.100.17',
  'For=192.0.2.43;proto=https;by=203.0.113.43',
  'for="[2001:db8:cafe::17]:4711"',
  'for=192.0.2.60;proto=http;by=203.0.113.43, for=198.51.100.17',
  'for=unknown;by=_hidden',
  'for=1.2.3.4;for=5.6.7.8',
  'for=""',
  'for="\\"quoted\\""',
  'for=',
  'for=1.2.3.4;proto',
  ' for=1.2.3.4',
  'for=1.2.3.4 ',
  // A quoted value holding a separator, which the old `name=value` rendering
  // could not tell apart from two parameters.
  'for=1.2.3.4;x="b,c=d"',
  'for=1.2.3.4;x=b;c=d',
  // RFC 7230 section 7: empty list elements must be ignored.
  'for=192.0.2.43,,for=198.51.100.7',
  'for=192.0.2.43,',
  ',for=192.0.2.43',
  'for=192.0.2.43, ,for=198.51.100.7'
]
for (const header of SEEDS) {
  const [strictness, expected] = row(header)
  lines.push(encode(header) + '\t' + strictness + '\t' + expected)
}

const TOTAL = 6000
for (let i = 0; i < TOTAL; i++) {
  let header
  const roll = rand()
  // Weighted towards well-formed headers: `noisy` and `corrupt` almost always
  // fail, so an even split would leave the accept path thin.
  if (roll < 0.6) header = structured()
  else if (roll < 0.85) header = corrupt(structured())
  else header = noisy()
  const [strictness, expected] = row(header)
  lines.push(encode(header) + '\t' + strictness + '\t' + expected)
}

fs.writeFileSync(OUT, lines.join('\n') + '\n')
console.log('wrote', lines.length - HEADER, 'cases to', OUT)
