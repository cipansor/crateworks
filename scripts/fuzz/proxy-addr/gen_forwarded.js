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
const FIXTURE = path.join(__dirname, '..', '..', '..', 'crates', 'proxy-addr',
  'tests', 'fixtures', 'forwarded.tsv')
const OUT = process.env.OUT || FIXTURE

// An unseen-seed replay that forgets `OUT` rewrites the tracked fixture with a
// corpus from a different seed, which looks like a normal run and silently
// re-baselines the corpus the pinned oracle exists to protect. Refuse instead.
if (process.env.SEED && !process.env.OUT) {
  console.error(
    'SEED is set but OUT is not, so this would overwrite the committed corpus ' +
    'with a different seed. Pass OUT=<path> for an unseen-seed replay, or ' +
    'unset SEED to regenerate ' + path.relative(process.cwd(), FIXTURE) + '.'
  )
  process.exit(1)
}

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
// strings. Each segment carries its start offset so the caller can tell which
// characters a rewrite removed.
function splitSegments (header) {
  const segments = []
  let start = 0
  let inQuotes = false
  let escaping = false
  for (let i = 0; i < header.length; i++) {
    const c = header[i]
    if (escaping) {
      escaping = false
    } else if (c === '\\' && inQuotes) {
      escaping = true
    } else if (c === '"') {
      inQuotes = !inQuotes
    } else if (c === ',' && !inQuotes) {
      segments.push({ text: header.slice(start, i), start })
      start = i + 1
      continue
    }
  }
  segments.push({ text: header.slice(start), start })
  return segments
}

// RFC 7230 section 7 requires a recipient to ignore empty list elements, and
// the reference does not, so a header the reference rejects for that reason is
// one this crate must still read. Rewriting the header without the empty
// segments gives a header the reference does accept, which is what proves the
// divergence is exactly that rule and nothing else.
//
// `#element => [ element ] *( OWS "," OWS [ element ] )` puts the whitespace on
// the separator, not on the element. So the whitespace that surrounded a
// dropped separator goes with it: `, for=a` is an empty element whose comma
// carries ` for=a`'s leading space, and leaving that space behind would produce
// ` for=a`, which the reference rejects for a reason that has nothing to do with
// empty elements. The same rule trims the whitespace before a dropped
// separator's comma (`for=a ,` -> `for=a`).
//
// The exception is the header's own first segment: whitespace there precedes
// any separator, so it is leading whitespace, which the list rule does not allow
// and the crate must still reject. It is kept.
//
// Returns the rewritten header and the offset of the first character it dropped
// (`Infinity` when it dropped nothing), because an error position before that
// offset is still the position in the original header.
function withoutEmptyElements (header) {
  const segments = splitSegments(header)
  const last = segments.length - 1
  // Only the header's own first segment is judged unstripped: whitespace there
  // precedes any separator, so it is leading whitespace and a fault of its own.
  const isEmpty = (i) => (i === 0
    ? segments[i].text === ''
    : segments[i].text.replace(/^[ \t]+|[ \t]+$/g, '') === '')
  const kept = []
  for (let i = 0; i < segments.length; i++) if (!isEmpty(i)) kept.push(i)

  let firstDropped = Infinity
  const dropped = (offset) => {
    if (offset >= 0 && offset < firstDropped) firstDropped = offset
  }
  let lastEmpty = -1
  for (let i = 0; i <= last; i++) {
    if (!isEmpty(i)) continue
    // The whole segment goes, and so does the comma that delimited it: the one
    // before it, or - for an empty first segment - the one after it.
    dropped(i === 0 ? segments[i].start : segments[i].start - 1)
    lastEmpty = i
  }

  const out = kept.map((i, position) => {
    let text = segments[i].text
    // Whitespace next to a dropped segment is that separator's OWS, so it goes
    // with it. Whitespace next to a surviving separator is OWS of that comma
    // and stays.
    const previous = position === 0 ? -1 : kept[position - 1]
    const next = position === kept.length - 1 ? last + 1 : kept[position + 1]
    if (i !== 0 && previous !== i - 1) {
      const lead = text.replace(/^[ \t]+/, '')
      if (lead.length !== text.length) dropped(segments[i].start)
      text = lead
    }
    if (next !== i + 1) {
      const trail = text.replace(/[ \t]+$/, '')
      if (trail.length !== text.length) {
        dropped(segments[i].start + trail.length)
      }
      text = trail
    }
    return text
  })
  // True when every removed element sat before the first surviving one. The
  // crate skips a comma only at an element boundary, with nothing collected
  // yet, so a *leading* run of empty elements is the one shape where it
  // provably reads the header exactly as it reads the stripped text. An empty
  // element after a kept one can follow partial text (`for=,`), where the crate
  // faults at the comma itself instead of skipping it.
  const leadingOnly = kept.length > 0 && lastEmpty < kept[0]
  return { text: out.join(','), firstDropped, leadingOnly }
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
//             reject it.
//
//             A bare `!` is the weak form, and it is only used when the index
//             cannot be compared. Removing an element shifts every position
//             after it, but an error before the first removed character keeps
//             its index, and dropping it would throw away a comparison for
//             nothing: ` for=a,,for=b` fails at index 0 for its leading space
//             whether or not the empty element is gone. When the reference's
//             index on the rewritten header is the same in both strings, it is
//             recorded as `!pos:<n>` and compared exactly.
//
//             A rejected remainder is otherwise compared by its shape, because
//             that much needs no translation: `!eof` is compared when the crate
//             provably skips exactly the commas that were removed, which is when
//             every removed element was leading (see `withoutEmptyElements`).
//             `",#="` -> `"#="` is that case and now demands `!eof`. Every other
//             rejected remainder keeps the bare `!`, which therefore means "not
//             compared", not "moved": an index can have moved, or the header
//             can strip to nothing (`","`), where the crate reports "no
//             element" with a position rather than truncation.
function row (header) {
  const direct = verdict(header)
  if (!direct.startsWith('!')) return ['strict', direct]
  const { text: stripped, firstDropped, leadingOnly } =
    withoutEmptyElements(header)
  if (stripped === header) return ['strict', direct]
  const repaired = verdict(stripped)
  if (!repaired.startsWith('!')) return ['empty', repaired]
  // `!eof` carries no index, so there is nothing to translate and it can be
  // compared - but only when the crate reads both strings the same way. That
  // holds when every removed element was leading: the commas the crate skips
  // are exactly those, so it arrives at the surviving text in the same state
  // and reports the same truncation. Any other removed element may have been a
  // comma the crate faults on (`for=,` faults at index 4, while `for=` is
  // merely truncated), so that shape keeps the weak verdict.
  if (repaired === '!eof') return ['empty', leadingOnly ? '!eof' : '!']
  const at = repaired.startsWith('!pos:') ? Number(repaired.slice(5)) : null
  // The index is measured in UTF-16 code units, and `firstDropped` is one too,
  // so the two are directly comparable.
  if (at !== null && at < firstDropped) return ['empty', repaired]
  return ['empty', '!']
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
  'for=192.0.2.43, ,for=198.51.100.7',
  // The whitespace around a dropped separator is that separator's OWS and must
  // go with it, or the rewrite leaves ` for=...`, which the reference rejects
  // for its leading space and the crate accepts.
  ', for=192.0.2.43',
  'for=192.0.2.43 ,,for=198.51.100.7',
  'for=192.0.2.43 ,',
  // An error before the first dropped character keeps its index, so the row can
  // still compare the position rather than only the rejection.
  ' for=192.0.2.43,,for=198.51.100.7'
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
