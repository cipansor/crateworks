'use strict'

// Corpus generator for the `fast-redact` crate.
//
// It asks the reference implementation, the npm package `fast-redact`, for the
// answer to a few thousand inputs a human would not think to try, and writes
// those answers to `crates/fast-redact/tests/fixtures/`. The crate's
// `tests/oracle.rs` then has to reproduce every one.
//
// Run it with NODE_PATH pointing at a `node_modules` that holds `fast-redact`:
//
//   NODE_PATH=<modules> node scripts/fuzz/fast-redact/gen.js [fixture-dir]
//
// The fixture directory defaults to the crate's own `tests/fixtures`. Two files
// are written: `fuzz.tsv` (documents, paths, and the reference's output) and
// `paths.tsv` (paths and whether the reference accepts them).
//
// The reference is MIT-licensed and is a development tool here; it is never a
// dependency of the crate and is never installed by CI.

const fs = require('fs')
const path = require('path')
const fastRedact = require('fast-redact')

// A small xorshift, so the corpus is identical on every run.
let state = 0x9e3779b9
function rnd () {
  state ^= state << 13
  state ^= state >>> 17
  state ^= state << 5
  state >>>= 0
  return state / 0x100000000
}
function pick (list) {
  return list[Math.floor(rnd() * list.length)]
}
function chance (p) {
  return rnd() < p
}

const KEYS = ['a', 'b', 'c', 'd', 'user', 'users', 'items', 'token', 'cookie',
  'authorization', 'password', 'secret', 'nested', 'deep', 'x-forwarded-for',
  'X-Forwarded-For', '', '0', '1', 'b.c', 'k0', 'k1']
const NUMBERS = [0, 1, -1, 42, 3.5, -0.25, 1e21, 1e-7, 123456789012345680000,
  0.1, 1e100, 5e-324, 2 ** 53, 9007199254740993, 1.7976931348623157e308]

function randomValue (depth) {
  const r = rnd()
  if (depth <= 0 || r < 0.45) {
    const s = rnd()
    if (s < 0.3) return pick(NUMBERS)
    if (s < 0.55) return 'v' + Math.floor(rnd() * 10)
    if (s < 0.7) return null
    if (s < 0.8) return chance(0.5)
    if (s < 0.9) return ''
    return pick(['secret', 'hunter2', 'a-b', 'x y', 'é', '\u0000\u001f', '"q"'])
  }
  if (r < 0.75) {
    const object = {}
    const n = 1 + Math.floor(rnd() * 3)
    for (let i = 0; i < n; i++) object[pick(KEYS)] = randomValue(depth - 1)
    return object
  }
  const array = []
  const n = Math.floor(rnd() * 4)
  for (let i = 0; i < n; i++) array.push(randomValue(depth - 1))
  return array
}

function segmentText (key) {
  // Bracket notation for a key that is not an identifier, dot notation otherwise.
  if (key === '') return '[""]'
  if (/^[A-Za-z_$][A-Za-z0-9_$]*$/.test(key)) return '.' + key
  return '["' + key.replace(/\\/g, '\\\\').replace(/"/g, '\\"') + '"]'
}

function randomPath (depth) {
  const n = 1 + Math.floor(rnd() * depth)
  let out = ''
  for (let i = 0; i < n; i++) {
    if (i > 0 && chance(0.35)) {
      out += '[' + Math.floor(rnd() * 4) + ']'
    } else if (chance(0.12)) {
      out += '[*]'
    } else if (chance(0.08) && i > 0) {
      out += '.*'
    } else {
      out += segmentText(pick(KEYS))
    }
  }
  return out
}

// Paths that exercise the documented shapes and the reference's own examples.
const FIXED_PATHS = [
  '*', '*.*', '*.*.*', 'a.*', 'a.*.b', 'a[*]', 'a[*].b', 'a[0]', 'a[0].b',
  'users.*.token', 'users[*].token', 'headers.cookie', 'headers.referer',
  'headers["x-forwarded-for"]', 'headers["X-Forwarded-For"]',
  'a.b.c', 'a.b.c.d', 'a[""]["y"]', '[*]["y"]', 'a["b.c"]', 'a["*"]',
  'a["x-y"]', 'a[1].b', 'nested.deep.secret', 'a.b.*', 'a.*.*', 'a.*.*.*',
  '*.c', '*.c.d', 'a[*].c.d', '*.*.x', '*.*.*.c', 'a.*.x.*.c', 'a.*.*.c',
  'insideArray.like[3].this.*', 'deep.bar.baz.ding.*', 'array[*]'
]

function writeFuzz (out) {
  const lines = []
  const seen = new Set()
  const modes = ['default', 'remove']
  let made = 0
  while (made < 6000) {
    const value = randomValue(3)
    if (value === null || typeof value !== 'object') continue
    const mode = pick(modes)
    const paths = chance(0.75)
      ? [chance(0.7) ? randomPath(4) : pick(FIXED_PATHS)]
      : [pick(FIXED_PATHS), randomPath(3)]
    const input = JSON.stringify(value)
    let output
    try {
      const redact = fastRedact({ paths, remove: mode === 'remove' })
      output = redact(JSON.parse(input))
    } catch (e) {
      continue
    }
    const key = mode + '\t' + JSON.stringify(paths) + '\t' + input
    if (seen.has(key)) continue
    seen.add(key)
    lines.push([mode, JSON.stringify(paths), input, output].join('\t'))
    made++
  }
  fs.writeFileSync(out, lines.join('\n') + '\n')
  return lines.length
}

const PATH_POOL = [
  'a', 'a.b', 'a.b.c', 'a[0]', 'a[1].b', 'a["x"]', 'a["x-y"]', 'a[""]',
  'a[*]', 'a.*', '*', '*.b', '[*]', '["a"]', '["a"].b', 'a["b.c"]',
  'a["*"]', 'a[0][1]', 'a[-1]', 'a[1.2]', 'a[0x1]', 'a[1e2]', 'a[+1]',
  'a[.5]', 'a[1.]', 'a[true]', 'a[null]', 'a[undefined]', 'a[NaN]',
  'a[Infinity]', 'a[`x`]', "a['x']", 'a[""]["y"]', '[*]["y"]', 'a[*].b',
  'a[0].*', '*[0]', 'a.*.b', '*.*', '*.*.*',
  'x-y', 'a.x-y', '@', '0', 'a.0', 'a..c', '1..c', 'a = b', 'a(b)',
  '//a.b.c', '\\a.b.c', 'a.#.c', '~~a.b.c', '^a.b.c', 'a + b', 'a / b',
  'a * b', 'a - b', 'a ** b', 'a % b', 'a.b*.c', 'a;global', 'a//',
  'a/*foo*/', 'a,o.b', 'a\n', 'a\r', 'a b', '_x', '$x', 'a$b', 'a_b', 'é',
  'a.é', 'a[é]', 'a[b]', '0a', 'a[()]', 'a[o]', 'a[this]', 'a[Math]',
  'a[1+1]', 'a[[]]', 'a[**]', 'a[ * ]', '', '.', '..', 'a.', '.a'
]

function writePaths (out) {
  const lines = []
  for (const p of PATH_POOL) {
    let verdict = 'accept'
    try {
      fastRedact({ paths: [p], serialize: false })
    } catch (e) {
      verdict = 'reject'
    }
    lines.push(verdict + '\t' + JSON.stringify(p))
  }
  fs.writeFileSync(out, lines.join('\n') + '\n')
  return lines.length
}

const dir = process.argv[2] || path.join(__dirname, '..', '..', '..',
  'crates', 'fast-redact', 'tests', 'fixtures')
fs.mkdirSync(dir, { recursive: true })
const fuzzCount = writeFuzz(path.join(dir, 'fuzz.tsv'))
const pathCount = writePaths(path.join(dir, 'paths.tsv'))
console.log('wrote ' + fuzzCount + ' redaction cases and ' + pathCount +
  ' path cases to ' + dir)
