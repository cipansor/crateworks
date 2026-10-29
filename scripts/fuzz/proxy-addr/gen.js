#!/usr/bin/env node
/*
 * Corpus generator for `crates/proxy-addr`.
 *
 * Drives the npm `proxy-addr` package (with the `forwarded` and `ipaddr.js`
 * libraries it builds on) over a generated corpus of requests and trust
 * specifications, and writes `crates/proxy-addr/tests/fixtures/oracle.tsv`.
 *
 * The oracle is the reference itself, so a disagreement is a bug in the crate.
 * Three things are recorded per case:
 *
 *   FWD  peer \t xff \t a \t b \t ...   the raw forwarded list, peer first
 *   ALL  peer \t xff \t trust \t ...    the chain up to the first untrusted hop
 *   ADDR peer \t xff \t trust \t result the address to attribute the request to
 *
 * A field that the reference throws on is written as `!<message>` so the crate
 * is checked to reject the same inputs, not merely to agree on the accepted
 * ones.
 *
 * Needs `npm install proxy-addr` in the repository root (the manifest is
 * committed). Run from the repository root:
 *
 *   node scripts/fuzz/proxy-addr/gen.js
 */

'use strict'

const fs = require('fs')
const path = require('path')
const proxyaddr = require('proxy-addr')
const forwarded = require('forwarded')

// `OUT` and `SEED` let the verification gate write a fresh corpus to a
// temporary file and leave the committed one alone.
const OUT = process.env.OUT || path.join(__dirname, '..', '..', '..', 'crates',
  'proxy-addr', 'tests', 'fixtures', 'oracle.tsv')

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

// The crate's API takes the peer as an `IpAddr`, so every peer here must parse
// as one. Header entries are left as text, so the untrustworthy shapes live in
// HOP instead.
const PEERS = [
  '10.0.0.5', '10.1.2.3', '172.16.4.4', '192.168.1.1', '127.0.0.1', '::1',
  '203.0.113.9', '8.8.8.8', '198.51.100.7', '::ffff:10.0.0.5', 'fe80::1',
  '2001:db8::1', '169.254.1.1', '100.64.0.1', '0.0.0.0', '255.255.255.255',
  '::ffff:192.168.0.1', '::ffff:8.8.8.8'
]

const HOP = [
  '1.2.3.4', '5.6.7.8', '10.0.0.5', '10.9.9.9', '172.16.4.4', '192.168.0.1',
  '198.51.100.7', '203.0.113.9', '8.8.8.8', '::1', 'fe80::1', '::ffff:10.0.0.5',
  '2001:db8::1', 'unknown', '_hidden', '', 'garbage', '1.2.3.4:5678',
  // IPv6 zone identifiers: the reference accepts these and ignores the zone.
  'fe80::1%eth0', 'fe80::1%1', '::1%eth0', '::ffff:10.0.0.5%eth0', 'fe80::1%',
  'fe80::1%et-h0', '1.2.3.4%eth0', '::ffff:10.0.0.5%eth0'
]

const TRUSTS = [
  'loopback', 'linklocal', 'uniquelocal',
  '10.0.0.0/8', '10.0.0.0', '172.16.0.0/12', '192.168.0.0/16', '127.0.0.0/8',
  '::1', '::1/128', 'fe80::/10', 'fc00::/7', '0.0.0.0/0', '0.0.0.0/33',
  '10.0.0.0/255.0.0.0', '10.0.0.0/255.0.255.0', '::ffff:10.0.0.0/104',
  '::ffff:0.0.0.0/96', '2001:db8::/32', '8.8.8.0/24', 'not-an-ip', '',
  '999.1.1.1/8', '10.0.0.0/-1', '10.0.0.0/abc',
  // A zone on a network entry: the reference strips it before parsing.
  'fe80::%eth0/10', 'fe80::1%eth0', '::ffff:10.0.0.0%x/104'
]

// A random CIDR, to reach prefixes and families the hand-written list misses.
function randomCidr() {
  if (rand() < 0.5) {
    const oct = () => Math.floor(rand() * 256)
    const prefix = Math.floor(rand() * 33)
    return `${oct()}.${oct()}.${oct()}.${oct()}/${prefix}`
  }
  const hex = () => Math.floor(rand() * 0x10000).toString(16)
  const prefix = Math.floor(rand() * 129)
  return `${hex()}:${hex()}::${hex()}/${prefix}`
}

// Trust lists combine one to four single specs, some of them random.
function randomTrustList() {
  const n = 1 + Math.floor(rand() * 4)
  const parts = []
  for (let i = 0; i < n; i++) {
    parts.push(rand() < 0.35 ? randomCidr() : pick(TRUSTS))
  }
  return parts
}

function randomXff() {
  const n = Math.floor(rand() * 5)
  if (n === 0) return null
  const parts = []
  for (let i = 0; i < n; i++) parts.push(pick(HOP))
  const sep = rand() < 0.15 ? ',' : ', '
  return parts.join(sep)
}

// A real request always carries a headers object; `null` here means the
// `X-Forwarded-For` header is absent, which the reference reads as `''`.
function makeReq(peer, xff) {
  const req = { socket: { remoteAddress: peer }, headers: {} }
  if (xff !== null) req.headers['x-forwarded-for'] = xff
  return req
}

// A throw is recorded so the crate is checked to reject the same input.
function attempt(fn) {
  try {
    return fn()
  } catch (e) {
    return '!' + e.message
  }
}

// A `FWD` or `ALL` value is a list of addresses, joined by tabs; an error is a
// single string. Both are escaped element-wise so the tab-separated line
// protocol stays unambiguous.
function listField(value) {
  if (typeof value === 'string' && value.startsWith('!')) return esc(value)
  return value.map(esc).join('\t')
}

function esc(s) {
  if (s === null || s === undefined) return '\\N'
  let out = String(s).replace(/\\/g, '\\\\').replace(/\t/g, '\\t').replace(/\n/g, '\\n')
  // A trailing space is invisible in a TSV and some tools strip it on save, so
  // it is encoded. It arises from the reference's message for an empty address.
  if (out.endsWith(' ')) out = out.slice(0, -1) + '\\x20'
  return out
}

const lines = []
lines.push('# Generated by scripts/fuzz/proxy-addr/gen.js from npm proxy-addr.')
lines.push('# FWD: peer, xff, then every forwarded address (peer first).')
lines.push('# ALL: peer, xff, trust-list, then the chain to the first untrusted hop.')
lines.push('# ADDR: peer, xff, trust-list, then the resolved client address.')

// --- FWD: the raw forwarded list, independent of any trust spec ---
for (const peer of PEERS) {
  for (const xff of [null, '', '1.2.3.4', '1.2.3.4, 5.6.7.8', 'a, , b', '  ',
    '1.2.3.4,,5.6.7.8', 'a  ,b', 'a\tb,c', '10.0.0.1, 10.0.0.2, 10.0.0.3']) {
    const got = attempt(() => forwarded(makeReq(peer, xff)))
    lines.push(['FWD', esc(peer), esc(xff), listField(got)].join('\t'))
  }
}

// --- Corners: IPv4-mapped matching, netmasks and single-bit prefixes ---
// The random pool reaches these rarely, and they are where a hand-written port
// is most likely to differ from `ipaddr.js`, so each is named.
const CORNERS = [
  ['1.2.3.4', '::ffff:1.2.3.0/120'],
  ['::ffff:1.2.3.4', '1.2.3.0/24'],
  ['1.2.3.4', '::ffff:0.0.0.0/96'],
  ['1.2.3.4', '::ffff:0.0.0.0/95'],
  ['::ffff:1.2.3.4', '::ffff:1.2.3.0/120'],
  ['10.0.0.1', '::ffff:10.0.0.0/104'],
  ['::ffff:10.0.0.1', '10.0.0.0/8'],
  ['::ffff:10.0.0.1', '::ffff:10.0.0.0/104'],
  ['::ffff:8.8.8.8', '8.8.8.0/24'],
  ['8.8.8.8', '::ffff:8.8.8.0/120'],
  ['127.0.0.1', '0.0.0.0/1'],
  ['127.0.0.1', '128.0.0.0/1'],
  ['10.1.1.1', '10.0.0.0/255.0.0.0'],
  ['10.1.1.1', '10.0.0.0/255.0.255.0'],
  ['::1', '::1/128'],
  ['2001:db8::1', '2001:db8::/32'],
  ['fe80::1', 'fe80::/10'],
  ['fc00::1', 'fc00::/7']
]
// The header carries one hop that no corner spec trusts, so `ADDR` is that hop
// when the peer is trusted and the peer itself when it is not: the result reads
// out the trust predicate directly.
const CORNER_HOP = '203.0.113.9'
for (const [peer, spec] of CORNERS) {
  const req = makeReq(peer, CORNER_HOP)
  const trust = [spec]
  const trustField = esc(spec)
  const all = attempt(() => proxyaddr.all(req, trust))
  lines.push(['ALL', esc(peer), esc(CORNER_HOP), trustField, listField(all)].join('\t'))
  lines.push(['ADDR', esc(peer), esc(CORNER_HOP), trustField,
    esc(attempt(() => proxyaddr(req, trust)))].join('\t'))
}

// --- ALL and ADDR: the resolution, over random trust lists ---
// `CASES` lets a one-off run widen the corpus to hunt for divergences without
// changing the committed fixture.
const CASES = Number(process.env.CASES || 6000)
for (let i = 0; i < CASES; i++) {
  const peer = pick(PEERS)
  const xff = randomXff()
  const trust = randomTrustList()
  const trustField = esc(trust.join('|'))
  const req = makeReq(peer, xff)

  const all = attempt(() => proxyaddr.all(req, trust))
  lines.push(['ALL', esc(peer), esc(xff), trustField, listField(all)].join('\t'))

  const addr = attempt(() => proxyaddr(req, trust))
  lines.push(['ADDR', esc(peer), esc(xff), trustField, esc(addr)].join('\t'))
}

fs.mkdirSync(path.dirname(OUT), { recursive: true })
fs.writeFileSync(OUT, lines.join('\n') + '\n')

const counts = lines.reduce((acc, l) => {
  if (l.startsWith('#')) return acc
  acc[l.split('\t')[0]] = (acc[l.split('\t')[0]] || 0) + 1
  return acc
}, {})
console.log(`wrote ${OUT}`)
console.log(counts)
