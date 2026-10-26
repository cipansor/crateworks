//! Rebuilds parse-address's own pattern strings and lookup tables, exactly as
//! its `lazyInit()` builds them, and writes them as Rust source.
//
// The parser is a single backtracking regular expression with named capture
// groups. Rust's `regex` cannot express it (it has no lookahead and no
// backtracking), and this workspace's crates depend on the standard library
// alone, so the crate carries a small backtracking engine instead. Embedding the
// reference's *own* pattern text, rather than re-deriving the grammar by hand,
// is what makes the port faithful: the grammar is data here, not a translation.
//
// The patterns are written in JavaScript's extended mode (`x`), where
// unescaped whitespace is ignored and `#` starts a comment. That mode is
// stripped here so the engine only has to understand the `i` flag.
//
// Usage: NODE_PATH=<modules with parse-address> node extract_patterns.js <out.rs>

const fs = require('fs');
const XRegExp = require('xregexp');

const Directional = {
  north: 'N', northeast: 'NE', east: 'E', southeast: 'SE',
  south: 'S', southwest: 'SW', west: 'W', northwest: 'NW',
};

// The USPS street-type table, keyed by every spelling that maps to a code.
const Street_Type = require('./street_type.json');
const State_Code = require('./state_code.json');

function keys(o) { return Object.keys(o); }
function values(o) { return keys(o).map((k) => o[k]); }
function invert(o) { const r = {}; keys(o).forEach((k) => { r[o[k]] = k; }); return r; }
function flatten(o) { return keys(o).concat(values(o)); }

const Direction_Code = invert(Directional);

const type = flatten(Street_Type).sort().filter((v, i, a) => a.indexOf(v) === i).join('|');
const direct = values(Directional)
  .sort((a, b) => a.length < b.length)
  .reduce((prev, curr) => prev.concat([XRegExp.escape(curr.replace(/\w/g, '$&.')), curr]), keys(Directional))
  .join('|');
const dircode = keys(Direction_Code).join('|');
const state = '\\b(?:' + keys(State_Code).concat(values(State_Code)).map(XRegExp.escape).join('|') + ')\\b';

const number = '(?<number>(\\d+-?\\d*)|([N|S|E|W]\\d{1,3}[N|S|E|W]\\d{1,6}))(?=\\D)';
const fraction = '\\d+\\/\\d+';
const zip = '(?<zip>\\d{5})[- ]?(?<plus4>\\d{4})?';
const corner = '(?:\\band\\b|\\bat\\b|&|\\@)';
const po_box = 'p\\W*(?:[om]|ost\\ ?office)\\W*b(?:ox)?';

const street = `
      (?:
        (?:(?<street_0>${direct})\\W+
           (?<type_0>${type})\\b
        )
        |
        (?:(?<prefix_0>${direct})\\W+)?
        (?:
          (?<street_1>[^,]*\\d)
          (?:[^\\w,]*(?<suffix_1>${direct})\\b)
          |
          (?<street_2>[^,]+)
          (?:[^\\w,]+(?<type_2>${type})\\b)
          (?:[^\\w,]+(?<suffix_2>${direct})\\b)?
          |
          (?<street_3>[^,]+?)
          (?:[^\\w,]+(?<type_3>${type})\\b)?
          (?:[^\\w,]+(?<suffix_3>${direct})\\b)?
        )
      )`;

const sec_unit_type_numbered = `
      (?<sec_unit_type_1>su?i?te
        |${po_box}
        |(?:ap|dep)(?:ar)?t(?:me?nt)?
        |ro*m
        |flo*r?
        |uni?t
        |bu?i?ldi?n?g
        |ha?nga?r
        |lo?t
        |pier
        |slip
        |spa?ce?
        |stop
        |tra?i?le?r
        |box)(?![a-z]
      )
      `;

const sec_unit_type_unnumbered = `
      (?<sec_unit_type_2>ba?se?me?n?t
        |fro?nt
        |lo?bby
        |lowe?r
        |off?i?ce?
        |pe?n?t?ho?u?s?e?
        |rear
        |side
        |uppe?r
      )\\b`;

const sec_unit = `
      (?:
        (?:
          (?:
            (?:${sec_unit_type_numbered}\\W*)
            |(?<sec_unit_type_3>\\#)\\W*
          )
          (?<sec_unit_num_1>[\\w-]+)
        )
        |
        ${sec_unit_type_unnumbered}
      )`;

const city_and_state = `
      (?:
        (?<city>[^\\d,]+?)\\W+
        (?<state>${state})
      )
      `;

const place = `
      (?:${city_and_state}\\W*)?
      (?:${zip})?
      `;

const address = `
      ^
      [^\\w\\#]*
      (${number})\\W*
      (?:${fraction}\\W*)?
         ${street}\\W+
      (?:${sec_unit})?\\W*
         ${place}
      \\W*$`;

const sep = '(?:\\W+|$)';
const informal_address = `
      ^
      \\s*
      (?:${sec_unit}${sep})?
      (?:${number})?\\W*
      (?:${fraction}\\W*)?
         ${street}${sep}
      (?:${sec_unit.replace(/_\d/g, '$&1')}${sep})?
      (?:${place})?
      `;

const po_address = `
      ^
      \\s*
      (?:${sec_unit.replace(/_\d/g, '$&1')}${sep})?
      (?:${place})?
      `;

const intersection = `
      ^\\W*
      ${street.replace(/_\d/g, '1$&')}\\W*?
      \\s+${corner}\\s+
      ${street.replace(/_\d/g, '2$&')}\\W+
      ${place}\\W*$`;

// Extended mode (`x`): drop a `#` comment to end of line, and drop unescaped
// whitespace. `\ ` is an escaped space, so it survives as a literal space.
function stripExtended(pattern) {
  let out = '';
  let inClass = false;
  for (let i = 0; i < pattern.length; i++) {
    const c = pattern[i];
    if (c === '\\') {
      out += c + (pattern[i + 1] || '');
      i++;
      continue;
    }
    if (c === '[') inClass = true;
    else if (c === ']') inClass = false;
    if (!inClass && c === '#') {
      while (i < pattern.length && pattern[i] !== '\n') i++;
      continue;
    }
    if (!inClass && /\s/.test(c)) continue;
    out += c;
  }
  return out;
}

// Only the patterns the crate names directly are emitted: the rest are
// sub-patterns already spliced into these, and an unused `pub(crate)` constant
// would be a dead-code warning under `-D warnings`.
const patterns = {
  corner, po_box, type,
  address, informal_address, po_address, intersection,
};

const stripped = {};
for (const [name, pattern] of Object.entries(patterns)) {
  stripped[name] = stripExtended(pattern);
}

// A sanity check against the real module: a stripped pattern compiled with only
// the `i` flag must accept and reject exactly what the module does on the
// fixture inputs.
const parser = require('parse-address');
const addressRe = XRegExp(stripped.address, 'i');
const fixture = fs.readFileSync(process.argv[3] || 'crates/address-parser/tests/fixtures/oracle.tsv', 'utf8');
let checked = 0;
for (const line of fixture.split('\n')) {
  if (!line) continue;
  const input = unescape(line.split('\t')[0]);
  const ours = addressRe.test(input);
  const theirs = parser.parseAddress(input) !== null;
  if (ours !== theirs) {
    throw new Error(`stripped pattern disagrees on ${JSON.stringify(input)}: ${ours} vs ${theirs}`);
  }
  checked++;
}
console.log('stripped address pattern agrees on', checked, 'inputs');

function unescape(text) {
  return text.replace(/\\(.)/g, (m, c) => ({ n: '\n', r: '\r', t: '\t', '\\': '\\', ';': ';', '=': '=' }[c] ?? c));
}

function rustString(text) {
  return 'r#"' + text + '"#';
}

const lines = [];
lines.push('//! The reference parser\'s own patterns, rebuilt by');
lines.push('//! `scripts/fuzz/address-parser/extract_patterns.js` and stripped of JavaScript\'s');
lines.push('//! extended (`x`) mode. Do not edit by hand: regenerate instead.');
lines.push('//!');
lines.push('//! The grammar lives here as data rather than as hand-written parsing code,');
lines.push('//! so a difference from the reference can only come from the engine that runs');
lines.push('//! these patterns, not from a mis-transcribed rule.');
lines.push('');
lines.push('');
for (const [name, pattern] of Object.entries(stripped)) {
  lines.push(`/// Pattern \`${name}\` from the reference.`);
  lines.push(`pub(crate) const ${name.toUpperCase()}: &str = ${rustString(pattern)};`);
  lines.push('');
}
lines.push('');
lines.push('/// The direction words the reference recognises, keyed by spelling.');
lines.push('pub(crate) const DIRECTIONAL: &[(&str, &str)] = &[');
for (const k of keys(Directional).sort()) lines.push(`    (${JSON.stringify(k)}, ${JSON.stringify(Directional[k])}),`);
lines.push('];');
lines.push('');
lines.push('/// The USPS street-type table, keyed by spelling.');
lines.push('pub(crate) const STREET_TYPE: &[(&str, &str)] = &[');
for (const k of keys(Street_Type).sort()) lines.push(`    (${JSON.stringify(k)}, ${JSON.stringify(Street_Type[k])}),`);
lines.push('];');
lines.push('');
lines.push('/// US state and territory names and codes.');
lines.push('pub(crate) const STATE_CODE: &[(&str, &str)] = &[');
for (const k of keys(State_Code).sort()) lines.push(`    (${JSON.stringify(k)}, ${JSON.stringify(State_Code[k])}),`);
lines.push('];');
lines.push('');
lines.push('/// Direction codes to their long form, the inverse of [`DIRECTIONAL`].');
lines.push('pub(crate) const DIRECTION_CODE: &[(&str, &str)] = &[');
for (const k of keys(Direction_Code).sort()) lines.push(`    (${JSON.stringify(k)}, ${JSON.stringify(Direction_Code[k])}),`);
lines.push('];');
lines.push('');

fs.writeFileSync(process.argv[2], lines.join('\n'));
console.log('wrote', process.argv[2], lines.length, 'lines');
