//! The public API: the reference's five entry points and its normalisation.

use std::collections::BTreeMap;

use crate::patterns;
use crate::regex::Regex;

/// One component of a parsed address.
///
/// The names are the reference's own field names, so a value can be looked up
/// the way the reference's output is read. An intersection uses the numbered
/// `Street1`/`Street2` and `Type1`/`Type2` forms instead of `Street`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Component {
    /// The house number, including a range or a fraction's whole part.
    Number,
    /// A leading directional, normalised to `N`, `NE` and so on.
    Prefix,
    /// The street name.
    Street,
    /// The street type, normalised to the USPS abbreviation and title-cased.
    Type,
    /// A trailing directional, normalised to `N`, `NE` and so on.
    Suffix,
    /// The unit kind, such as `Suite`, `Apt` or `PO Box`.
    SecUnitType,
    /// The unit number.
    SecUnitNum,
    /// The city or town.
    City,
    /// The state, normalised to the two-letter code.
    State,
    /// The five-digit ZIP code.
    Zip,
    /// The four-digit ZIP extension.
    Plus4,
    /// The first street of an intersection.
    Street1,
    /// The second street of an intersection.
    Street2,
    /// The first street's type in an intersection.
    Type1,
    /// The second street's type in an intersection.
    Type2,
    /// The first street's leading directional in an intersection.
    Prefix1,
    /// The second street's leading directional in an intersection.
    Prefix2,
    /// The first street's trailing directional in an intersection.
    Suffix1,
    /// The second street's trailing directional in an intersection.
    Suffix2,
}

impl Component {
    /// The reference's field name for this component.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Number => "number",
            Self::Prefix => "prefix",
            Self::Street => "street",
            Self::Type => "type",
            Self::Suffix => "suffix",
            Self::SecUnitType => "sec_unit_type",
            Self::SecUnitNum => "sec_unit_num",
            Self::City => "city",
            Self::State => "state",
            Self::Zip => "zip",
            Self::Plus4 => "plus4",
            Self::Street1 => "street1",
            Self::Street2 => "street2",
            Self::Type1 => "type1",
            Self::Type2 => "type2",
            Self::Prefix1 => "prefix1",
            Self::Prefix2 => "prefix2",
            Self::Suffix1 => "suffix1",
            Self::Suffix2 => "suffix2",
        }
    }

    /// The component with the reference's field name `name`, if there is one.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "number" => Self::Number,
            "prefix" => Self::Prefix,
            "street" => Self::Street,
            "type" => Self::Type,
            "suffix" => Self::Suffix,
            "sec_unit_type" => Self::SecUnitType,
            "sec_unit_num" => Self::SecUnitNum,
            "city" => Self::City,
            "state" => Self::State,
            "zip" => Self::Zip,
            "plus4" => Self::Plus4,
            "street1" => Self::Street1,
            "street2" => Self::Street2,
            "type1" => Self::Type1,
            "type2" => Self::Type2,
            "prefix1" => Self::Prefix1,
            "prefix2" => Self::Prefix2,
            "suffix1" => Self::Suffix1,
            "suffix2" => Self::Suffix2,
            _ => return None,
        })
    }
}

/// A parsed address: the components the reference found, and nothing else.
///
/// A component that the input did not carry is absent rather than empty, which
/// is the difference the reference draws and the reason this is not a struct of
/// `Option<String>` fields.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Address {
    parts: BTreeMap<Component, String>,
}

impl Address {
    /// The value of `component`, if the input carried it.
    #[must_use]
    pub fn get(&self, component: Component) -> Option<&str> {
        self.parts.get(&component).map(String::as_str)
    }

    /// Whether no component was found at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.parts.is_empty()
    }

    /// The components, in the reference's field-name order.
    pub fn iter(&self) -> impl Iterator<Item = (Component, &str)> {
        self.parts.iter().map(|(k, v)| (*k, v.as_str()))
    }

    /// The house number, if present.
    #[must_use]
    pub fn number(&self) -> Option<&str> {
        self.get(Component::Number)
    }

    /// The street name, if present.
    #[must_use]
    pub fn street(&self) -> Option<&str> {
        self.get(Component::Street)
    }

    /// The city, if present.
    #[must_use]
    pub fn city(&self) -> Option<&str> {
        self.get(Component::City)
    }

    /// The state, normalised to a two-letter code, if present.
    #[must_use]
    pub fn state(&self) -> Option<&str> {
        self.get(Component::State)
    }

    /// The five-digit ZIP code, if present.
    #[must_use]
    pub fn zip(&self) -> Option<&str> {
        self.get(Component::Zip)
    }

    fn insert(&mut self, component: Component, value: &str) {
        self.parts.insert(component, value.to_string());
    }
}

/// The patterns, compiled once on first use.
///
/// Compiling is not free - the intersection pattern alone is twenty thousand
/// characters, and building its program walks every alternative - so a caller
/// parsing a column of addresses must not pay for it on every line. The
/// patterns are immutable, so a single compiled copy is shared.
struct Patterns {
    address: Regex,
    informal: Regex,
    po: Regex,
    intersection: Regex,
    corner: Regex,
    po_box_start: Regex,
    type_only: Regex,
}

impl Patterns {
    fn new() -> Self {
        Self {
            address: Regex::compile(patterns::ADDRESS, true),
            informal: Regex::compile(patterns::INFORMAL_ADDRESS, true),
            po: Regex::compile(patterns::PO_ADDRESS, true),
            intersection: Regex::compile(patterns::INTERSECTION, true),
            corner: Regex::compile(patterns::CORNER, true),
            // `^` plus the PO box form, as the reference tests it.
            po_box_start: Regex::compile(&format!("^{}", patterns::PO_BOX), true),
            type_only: Regex::compile(&format!("^{}$", patterns::TYPE), true),
        }
    }

    fn get() -> &'static Self {
        static PATTERNS: std::sync::OnceLock<Patterns> = std::sync::OnceLock::new();
        PATTERNS.get_or_init(Patterns::new)
    }
}

/// Parses an address the way the reference's `parseLocation` does.
///
/// It tries an intersection, then a PO box, then a street address, then the
/// looser informal form, and returns the first that matches. `None` means
/// nothing matched, which is what the reference returns as `null`.
///
/// # Example
///
/// ```
/// use address_parser::{parse_location, Component};
///
/// let address = parse_location("1005 N Gravenstein Highway, Sebastopol, CA").unwrap();
/// assert_eq!(address.number(), Some("1005"));
/// assert_eq!(address.street(), Some("Gravenstein"));
/// assert_eq!(address.get(Component::Prefix), Some("N"));
/// assert_eq!(address.city(), Some("Sebastopol"));
/// assert_eq!(address.state(), Some("CA"));
/// ```
#[must_use]
pub fn parse_location(input: &str) -> Option<Address> {
    let patterns = Patterns::get();
    let chars: Vec<char> = input.chars().collect();
    if patterns.corner.is_match(&chars) {
        return parse_intersection_with(patterns, &chars);
    }
    if patterns.po_box_start.is_match(&chars) {
        return parse_po_with(patterns, &chars);
    }
    parse_address_with(patterns, &chars).or_else(|| parse_informal_with(patterns, &chars))
}

/// Parses a street address the way the reference's `parseAddress` does.
#[must_use]
pub fn parse_address(input: &str) -> Option<Address> {
    let patterns = Patterns::get();
    let chars: Vec<char> = input.chars().collect();
    parse_address_with(patterns, &chars)
}

/// Parses an address with no punctuation the way the reference's
/// `parseInformalAddress` does.
#[must_use]
pub fn parse_informal_address(input: &str) -> Option<Address> {
    let patterns = Patterns::get();
    let chars: Vec<char> = input.chars().collect();
    parse_informal_with(patterns, &chars)
}

/// Parses a post-office box address the way the reference's `parsePoAddress`
/// does.
#[must_use]
pub fn parse_po_address(input: &str) -> Option<Address> {
    let patterns = Patterns::get();
    let chars: Vec<char> = input.chars().collect();
    parse_po_with(patterns, &chars)
}

/// Parses an intersection the way the reference's `parseIntersection` does.
#[must_use]
pub fn parse_intersection(input: &str) -> Option<Address> {
    let patterns = Patterns::get();
    let chars: Vec<char> = input.chars().collect();
    parse_intersection_with(patterns, &chars)
}

fn parse_address_with(patterns: &Patterns, chars: &[char]) -> Option<Address> {
    let captures = patterns.address.exec(chars)?;
    Some(normalize(&captures, chars))
}

fn parse_informal_with(patterns: &Patterns, chars: &[char]) -> Option<Address> {
    let captures = patterns.informal.exec(chars)?;
    Some(normalize(&captures, chars))
}

fn parse_po_with(patterns: &Patterns, chars: &[char]) -> Option<Address> {
    let captures = patterns.po.exec(chars)?;
    Some(normalize(&captures, chars))
}

fn parse_intersection_with(patterns: &Patterns, chars: &[char]) -> Option<Address> {
    let captures = patterns.intersection.exec(chars)?;
    let mut address = normalize(&captures, chars);
    // An intersection always reports both type slots, empty when the input had
    // no street type, so `type1` and `type2` are never simply absent.
    let type1 = address.get(Component::Type1).unwrap_or("").to_string();
    let type2 = address.get(Component::Type2).unwrap_or("").to_string();
    // The reference fills both from the second when only the second is
    // non-empty, or when the two are equal, and strips a trailing plural `s`
    // when the result is a known street type.
    if !type2.is_empty() && (type1.is_empty() || type1 == type2) {
        let candidate = strip_trailing_plural(&type2);
        let probe: Vec<char> = candidate.chars().collect();
        if patterns.type_only.is_match(&probe) {
            address.insert(Component::Type1, &candidate);
            address.insert(Component::Type2, &candidate);
            return Some(address);
        }
    }
    address.insert(Component::Type1, &type1);
    address.insert(Component::Type2, &type2);
    Some(address)
}

/// Removes a trailing `s` that has only non-word characters after it.
///
/// This is the reference's `/s\W*$/` replacement, which turns `Rds` into `Rd`
/// and `Cts.` into `Ct`. A word that merely ends in `s` with nothing after it
/// also loses the `s`, so `Bays` becomes `Bay`.
fn strip_trailing_plural(value: &str) -> String {
    let chars: Vec<char> = value.chars().collect();
    let mut end = chars.len();
    while end > 0 && !is_word_char(chars[end - 1]) {
        end -= 1;
    }
    if end > 0 && chars[end - 1] == 's' {
        return chars[..end - 1].iter().collect();
    }
    value.to_string()
}

/// Whether `c` is a word character, matching the engine's `\w`.
fn is_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Turns a match into an [`Address`], applying the reference's normalization.
///
/// The reference's `normalize_address` does four things, in order: trims each
/// value and drops the characters it does not keep, folds a spelling through
/// its lookup table, title-cases a street type, and rewrites a leading direction
/// word in the city. All four are reproduced here.
fn normalize(captures: &crate::regex::Captures, chars: &[char]) -> Address {
    // The reference iterates a JavaScript object whose keys are the capture
    // names in declaration order, so a later group with the same base name
    // overwrites an earlier one. `Captures::named` preserves that.
    let mut address = Address::default();
    for (name, (start, end)) in captures.named() {
        // A group name ends in `_<digit>` when the pattern has alternatives that
        // capture the same field; the digit is stripped to get the field name.
        let base = match name.rsplit_once('_') {
            Some((head, tail)) if tail.chars().all(|c| c.is_ascii_digit()) && !tail.is_empty() => {
                head
            }
            _ => name.as_str(),
        };
        let Some(component) = Component::from_name(base) else {
            continue;
        };
        let value: String = chars[start..end].iter().collect();
        // The reference keeps a field when its capture is *truthy*, which is
        // false only for the empty string: a group that captured a single space
        // is kept and then trimmed to an empty value, and appears in the output.
        // So the test is on the raw text, not on the cleaned text.
        if value.is_empty() {
            continue;
        }
        address.insert(component, &clean(&value));
    }

    fold_spellings(&mut address);
    title_case_types(&mut address);
    fix_city_direction(&mut address);
    address
}

/// Trims `value` and keeps only the characters the reference keeps.
///
/// The reference strips everything outside `[\w\s\-#&]`, which is ASCII `\w`
/// plus whitespace, so a comma or a slash is dropped from a value.
fn clean(value: &str) -> String {
    let kept: String = value
        .chars()
        .filter(|c| {
            c.is_ascii_alphanumeric()
                || *c == '_'
                || c.is_whitespace()
                || matches!(c, '-' | '#' | '&')
        })
        .collect();
    kept.trim().to_string()
}

/// Folds a spelling through the reference's lookup tables.
///
/// Each field has its own table in the reference: a direction folds through
/// `Directional`, a street type through `Street_Type`, a state through
/// `State_Code`. The lookup is on the lower-cased value and only replaces when
/// the table has an entry, so `highway` becomes `Hwy`, `california` becomes
/// `CA`, and a value the table does not know is left as it was written.
fn fold_spellings(address: &mut Address) {
    for component in [
        Component::Prefix,
        Component::Prefix1,
        Component::Prefix2,
        Component::Suffix,
        Component::Suffix1,
        Component::Suffix2,
        Component::Type,
        Component::Type1,
        Component::Type2,
        Component::State,
    ] {
        let Some(value) = address.get(component) else {
            continue;
        };
        let lowered = value.to_lowercase();
        let table = match component {
            Component::Type | Component::Type1 | Component::Type2 => patterns::STREET_TYPE,
            Component::State => patterns::STATE_CODE,
            _ => patterns::DIRECTIONAL,
        };
        if let Some(replacement) = lookup(table, &lowered) {
            address.insert(component, replacement);
        }
    }
}

/// Title-cases a street type: `hwy` becomes `Hwy`, `HWY` becomes `Hwy`.
fn title_case_types(address: &mut Address) {
    for component in [Component::Type, Component::Type1, Component::Type2] {
        let Some(value) = address.get(component) else {
            continue;
        };
        let mut chars = value.chars();
        let Some(first) = chars.next() else { continue };
        let mut titled = String::new();
        titled.extend(first.to_uppercase());
        titled.push_str(&chars.as_str().to_lowercase());
        address.insert(component, &titled);
    }
}

/// Rewrites a leading direction word in the city, such as `North Springfield`.
///
/// The reference looks for a direction code at the start of the city, then one
/// or more whitespace characters, then a non-space, and replaces the code and
/// the whitespace with the long form title-cased. So `N  Springfield` becomes
/// `North Springfield` with a single space, and the rest of the city is left
/// exactly as it was written.
///
/// The code order does not matter: two codes cannot both be followed by
/// whitespace at the same position, because one is a prefix of the other and
/// would need the same character to be both whitespace and a letter.
fn fix_city_direction(address: &mut Address) {
    let Some(city) = address.get(Component::City).map(str::to_string) else {
        return;
    };
    // Only the direction word changes; the rest of the city keeps the case it
    // was written in, so the comparison is on an upper-cased copy and the
    // remainder comes from the original.
    let upper = city.to_uppercase();
    for code in ["N", "NE", "E", "SE", "S", "SW", "W", "NW"] {
        let Some(rest) = upper.strip_prefix(code) else {
            continue;
        };
        // `\s+` consumes every whitespace character, then `(?=\S)` requires a
        // non-space to follow, so trailing whitespace alone is not a match.
        let after = rest.trim_start_matches(char::is_whitespace);
        if after.len() == rest.len() || after.is_empty() {
            continue;
        }
        let Some(long) = lookup(patterns::DIRECTION_CODE, code) else {
            continue;
        };
        let mut rebuilt = capitalize(long);
        rebuilt.push(' ');
        rebuilt.push_str(&city[code.len() + (rest.len() - after.len())..]);
        address.insert(Component::City, &rebuilt);
        return;
    }
}

/// Upper-cases the first character and leaves the rest alone, which is the
/// reference's `capitalize`.
fn capitalize(value: &str) -> String {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    let mut out = String::new();
    out.extend(first.to_uppercase());
    out.push_str(chars.as_str());
    out
}

/// Looks up `key` in a two-column table.
fn lookup(table: &'static [(&'static str, &'static str)], key: &str) -> Option<&'static str> {
    table
        .iter()
        .find(|(name, _)| *name == key)
        .map(|(_, value)| *value)
}
