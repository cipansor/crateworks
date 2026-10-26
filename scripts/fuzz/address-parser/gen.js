//! Builds the address-parser corpus: free-text US addresses, each with the
//! components the reference parser extracts from it. Run with the
//! `parse-address` npm package on the module path. See scripts/fuzz/README.md.
//
// The reference is rule-based, so every case is reproducible from the script
// alone once the package is installed. The curated half comes from the
// reference's own test suite; the generated half composes addresses out of its
// own vocabulary, which is the part nobody would think to write down.

const fs = require('fs');
const parser = require('parse-address');

// A seeded PRNG, so the corpus is the same on every run. Math.random would make
// the fixture impossible to rebuild from the committed script.
let state = 0x1f2e3d4c;
function random() {
  state ^= state << 13;
  state ^= state >>> 17;
  state ^= state << 5;
  state >>>= 0;
  return state / 0x100000000;
}
function pick(items) {
  return items[Math.floor(random() * items.length)];
}

// Vocabulary drawn from the reference's own tables and test suite.
const NUMBERS = ['1', '12', '123', '1005', '1600', '42', '7', '99999', '12-14', '9'];
const DIRECTIONS = ['N', 'S', 'E', 'W', 'NE', 'NW', 'SE', 'SW', 'North', 'South'];
const TYPES = ['St', 'Street', 'Ave', 'Avenue', 'Rd', 'Road', 'Blvd', 'Hwy', 'Highway',
  'Ln', 'Dr', 'Drive', 'Ct', 'Pl', 'Way', 'Loop', 'Pkwy', 'Ter', 'Cir', 'Run'];
const STREETS = ['Main', 'Oak', 'Gravenstein', 'Pennsylvania', 'Elm', 'Maple', 'First',
  'Infinite', 'Coyote', 'Maison', 'Wall', 'London', 'Pine', 'Cedar', 'Washington',
  'Lake', 'Hill', 'Park', 'Chestnut', 'Walnut'];
const CITIES = ['Sebastopol', 'Springfield', 'Austin', 'Dallas', 'Boston', 'Cupertino',
  'Hattiesburg', 'Fishers', 'Bloomfield Hills', 'Portland', 'Denver', 'Tampa'];
const STATES = ['CA', 'IL', 'TX', 'MA', 'IN', 'MI', 'OR', 'CO', 'FL', 'NY', 'WA', 'AZ',
  'California', 'Texas', 'Illinois', 'Florida'];
const UNITS = [
  ['Suite', '100'], ['Suite', '200'], ['Apt', '4B'], ['Apt', '3'], ['#', '12'],
  ['Unit', '7'], ['Bldg', 'C'], ['Floor', '2'], ['Room', '5'], ['Ste', '9'],
  ['Trailer', '8'], ['Hangar', '1'], ['Lot', '3'], ['Pier', '2'], ['Slip', '4'],
];
const ZIP = ['95472', '62704', '75201', '02101', '95014', '39402', '46038', '48304',
  '97201', '80202', '33601', '10001'];
const UNIT_WORDS = ['Basement', 'Front', 'Lobby', 'Lower', 'Office', 'Penthouse',
  'Rear', 'Side', 'Upper'];

function build() {
  const parts = [pick(NUMBERS)];
  if (random() < 0.35) parts.push(pick(DIRECTIONS));
  parts.push(pick(STREETS));
  if (random() < 0.85) parts.push(pick(TYPES));
  if (random() < 0.3) parts.push(pick(DIRECTIONS));
  let line = parts.join(' ');

  if (random() < 0.35) {
    const [t, n] = pick(UNITS);
    line += t === '#' ? ' #' + n : ' ' + t + ' ' + n;
  }
  if (random() < 0.75) {
    line += ', ' + pick(CITIES);
    if (random() < 0.9) line += ', ' + pick(STATES);
    if (random() < 0.6) line += ' ' + pick(ZIP);
  } else if (random() < 0.5) {
    line += ' ' + pick(ZIP);
  }
  return line;
}

function buildPo() {
  const form = pick(['PO Box', 'P.O. Box', 'P O Box', 'Post Office Box', 'PO B']);
  let line = form + ' ' + pick(['1', '1234', '99', '500', '7']);
  if (random() < 0.8) {
    line += ', ' + pick(CITIES) + ', ' + pick(STATES);
    if (random() < 0.6) line += ' ' + pick(ZIP);
  }
  return line;
}

function buildIntersection() {
  // Both streets carry a directional often enough that the `prefix1`/`prefix2`
  // fields are exercised: they are the ones the reference only emits for an
  // intersection, so a corpus without them never tests that path.
  const street = () => {
    let s = '';
    if (random() < 0.4) s += pick(DIRECTIONS) + ' ';
    s += pick(STREETS) + ' ' + pick(TYPES);
    // A trailing directional belongs after the street type, which is the order
    // that makes the reference read it as `suffix` rather than part of the name.
    if (random() < 0.35) s += ' ' + pick(DIRECTIONS);
    return s;
  };
  const a = street();
  const b = street();
  const sep = pick([' and ', ' & ', ' @ ', ' at ']);
  let line = a + sep + b;
  if (random() < 0.7) {
    line += ', ' + pick(CITIES);
    if (random() < 0.8) line += ', ' + pick(STATES);
  }
  return line;
}

function buildUnitWord() {
  let line = pick(NUMBERS) + ' ' + pick(STREETS) + ' ' + pick(TYPES) + ' ' + pick(UNIT_WORDS);
  if (random() < 0.7) line += ', ' + pick(CITIES) + ', ' + pick(STATES);
  return line;
}

// The reference's own test table, transcribed rather than retyped: the file is
// read and the literal is evaluated, so a case cannot be mistyped.
const curated = [];
const testPath = require.resolve('parse-address/package.json').replace('package.json', 'test.js');
const testSrc = fs.readFileSync(testPath, 'utf8');
const tableMatch = testSrc.match(/var address = (\{[\s\S]*?\n\});/);
if (!tableMatch) {
  throw new Error('could not find the reference test table in ' + testPath);
}
for (const input of Object.keys(eval('(' + tableMatch[1] + ')'))) curated.push(input);

const inputs = curated.slice();
for (let i = 0; i < 4000; i++) {
  const r = random();
  if (r < 0.55) inputs.push(build());
  else if (r < 0.75) inputs.push(buildPo());
  else if (r < 0.9) inputs.push(buildIntersection());
  else inputs.push(buildUnitWord());
}

// Degenerate and adversarial inputs, where a parser's edge behaviour shows.
inputs.push('', '   ', '1005', 'garbage input that is not an address', '!!!',
  '5th Ave & Main St', '123 1/2 Main St, Boston, MA', '\t\n123 Main St\n',
  '123 MAIN ST, SPRINGFIELD, IL 62704', '123 main st springfield il',
  'PO Box 1234 Dallas TX', 'P.O. Box 1234, Dallas, TX 75201');

// The fixture is one case per line, and both columns are escaped so that an
// input containing a tab or a newline cannot break the layout: a backslash
// introduces a two-character escape, which is why the backslash itself is
// escaped first. Column two is `null`, or the components as `key=value` pairs
// sorted by key and joined with `;`, so the test does not have to parse JSON.
function escape(text) {
  return String(text)
    .replace(/\\/g, '\\\\')
    .replace(/\t/g, '\\t')
    .replace(/\n/g, '\\n')
    .replace(/\r/g, '\\r')
    .replace(/;/g, '\\;')
    .replace(/=/g, '\\=');
}

// Deduplicate, keeping first-seen order, and record the reference's answer.
const seen = new Set();
const rows = [];
for (const input of inputs) {
  if (seen.has(input)) continue;
  seen.add(input);
  let parsed;
  try {
    parsed = parser.parseLocation(input);
  } catch (e) {
    parsed = { __error: String(e && e.message) };
  }
  let answer = 'null';
  if (parsed) {
    answer = Object.keys(parsed)
      .sort()
      .map((key) => escape(key) + '=' + escape(parsed[key]))
      .join(';');
  }
  rows.push(escape(input) + '\t' + answer);
}

const out = process.argv[2] || 'crates/address-parser/tests/fixtures/oracle.tsv';
fs.writeFileSync(out, rows.join('\n') + '\n');
console.log('wrote', rows.length, 'cases to', out);
