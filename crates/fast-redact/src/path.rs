//! Path parsing: turning `a.b[*].c` into the segments the redactor walks.
//!
//! A path names a location in a JSON document. The syntax is the one npm
//! `fast-redact` documents — dot notation, bracket notation for keys that are
//! not identifiers, array indices, and `*` wildcards — and this module produces
//! the same segment list the reference produces for that syntax.
//!
//! # What is accepted, and one deliberate divergence
//!
//! The reference validates a path by compiling it as a JavaScript member
//! expression and evaluating it against a `Proxy`. That check exists because the
//! reference generates JavaScript source from the path, so an unescaped path is
//! code injection; `a;global.foo = "bar"` must be refused. This crate never
//! turns a path into code, so that risk does not exist here, and the check is
//! not reproduced.
//!
//! What is reproduced is the *documented* syntax. A path is a dotted identifier
//! followed by any number of `.identifier` or `[...]` groups. A bracket group
//! holds a quoted string, a number, or `*`. Anything else is refused.
//!
//! The consequence is that a few paths the reference accepts by accident —
//! because they happen to be JavaScript that evaluates without throwing — are
//! refused here: `a[Math]` (a global object), `a[this]`, `a[1+1]` (arithmetic),
//! `a[[]]` (an array literal). Every one names a key no JSON document can have,
//! so no redaction is lost. `tests/oracle.rs` asserts this list so the
//! divergence cannot grow unnoticed.

/// Why a path was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathError {
    /// The path that was rejected.
    pub path: String,
    /// What is wrong with it.
    pub message: String,
}

impl std::fmt::Display for PathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid path {:?}: {}", self.path, self.message)
    }
}

impl std::error::Error for PathError {}

/// A parsed path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Path {
    /// A path with no wildcard: every segment must be present for the leaf to
    /// be redacted.
    Literal(Vec<String>),
    /// A path containing a `*`. `before` is everything up to the first `*`,
    /// `after` is everything after it, and `nested` says whether anything
    /// follows the `*` at all (`a.*` does not; `a.*.b` does).
    ///
    /// Splitting at the first `*` is what the reference does, and it is why
    /// `*.*` behaves the way it does rather than as a general glob.
    Wildcard {
        /// Segments before the first `*`.
        before: Vec<String>,
        /// Segments after the first `*`.
        after: Vec<String>,
        /// Whether `after` is non-empty.
        nested: bool,
    },
}

/// Parse and validate a path.
///
/// ```
/// use fast_redact::{Path, parse_path};
///
/// assert_eq!(parse_path("a.b").unwrap(), Path::Literal(vec!["a".into(), "b".into()]));
/// assert!(matches!(parse_path("a[*].b"), Ok(Path::Wildcard { nested: true, .. })));
/// assert!(parse_path("a..b").is_err());
/// assert!(parse_path("a;o.b = 1").is_err());
/// ```
pub fn parse(path: &str) -> Result<Path, PathError> {
    let segments = tokenize(path)?;
    if segments.is_empty() {
        return Err(error(path, "the path names nothing"));
    }
    Ok(match segments.iter().position(|s| s == "*") {
        None => Path::Literal(segments),
        Some(star) => {
            let after = segments[star + 1..].to_vec();
            Path::Wildcard {
                before: segments[..star].to_vec(),
                nested: !after.is_empty(),
                after,
            }
        }
    })
}

fn error(path: &str, message: &str) -> PathError {
    PathError {
        path: path.to_string(),
        message: message.to_string(),
    }
}

/// Split a path into segments, refusing anything outside the documented syntax.
///
/// A `*` becomes a segment named `*` whether it was written `a.*` or `a[*]`,
/// which is what the reference does after it strips quotes: `a["*"]` is a
/// wildcard too.
fn tokenize(path: &str) -> Result<Vec<String>, PathError> {
    let chars: Vec<char> = path.chars().collect();
    let mut segments = Vec::new();
    let mut i = 0;
    let mut expect_segment = true;

    while i < chars.len() {
        match chars[i] {
            '.' => {
                if expect_segment {
                    return Err(error(path, "a `.` with no segment before it"));
                }
                expect_segment = true;
                i += 1;
            }
            '[' => {
                if expect_segment && !segments.is_empty() {
                    return Err(error(path, "a `[` with no `.` before it"));
                }
                let end = chars[i + 1..]
                    .iter()
                    .position(|c| *c == ']')
                    .map(|offset| i + 1 + offset)
                    .ok_or_else(|| error(path, "an unclosed `[`"))?;
                let content: String = chars[i + 1..end].iter().collect();
                segments.push(bracket_segment(path, &content)?);
                expect_segment = false;
                i = end + 1;
            }
            ']' => return Err(error(path, "a `]` with no matching `[`")),
            _ => {
                if !expect_segment {
                    return Err(error(path, "two segments with no `.` between them"));
                }
                let start = i;
                while i < chars.len() && !matches!(chars[i], '.' | '[' | ']') {
                    i += 1;
                }
                let text: String = chars[start..i].iter().collect();
                segments.push(dotted_segment(path, &text)?);
                expect_segment = false;
            }
        }
    }
    if expect_segment && !segments.is_empty() {
        return Err(error(path, "a trailing `.`"));
    }
    Ok(segments)
}

/// The value of a `[...]` group: a quoted string, a number, or `*`.
///
/// The reference strips `"`, `'` and `` ` `` from the whole path before it
/// splits it, so `a["x-y"]`, `a['x-y']` and ``a[`x-y`]`` all name the key
/// `x-y`, and `a[""]` names the empty key. A bracket with no quotes must hold a
/// number or a bare `*`; `a[ * ]` is not a wildcard, matching the reference,
/// where the spaces make it a syntax error.
fn bracket_segment(path: &str, content: &str) -> Result<String, PathError> {
    if content == "*" {
        return Ok("*".to_string());
    }
    if content.chars().any(is_quote) {
        return Ok(content.chars().filter(|c| !is_quote(*c)).collect());
    }
    let trimmed = content.trim();
    if is_number(trimmed) {
        return Ok(trimmed.to_string());
    }
    Err(error(
        path,
        "a bracket must hold a quoted string, a number, or `*`",
    ))
}

/// A dotted segment: an identifier, or `*`.
fn dotted_segment(path: &str, text: &str) -> Result<String, PathError> {
    if text == "*" {
        return Ok("*".to_string());
    }
    let mut chars = text.chars();
    match chars.next() {
        Some(first) if is_identifier_start(first) => {}
        _ => return Err(error(path, "a dotted segment must be an identifier")),
    }
    if chars.any(|c| !is_identifier_continue(c)) {
        return Err(error(path, "a dotted segment must be an identifier"));
    }
    Ok(text.to_string())
}

/// A JavaScript numeric literal: decimal with an optional fraction and exponent,
/// or a `0x`/`0o`/`0b` literal, with an optional sign.
fn is_number(text: &str) -> bool {
    let digits = text.strip_prefix(['+', '-']).unwrap_or(text);
    if let Some(hex) = digits
        .strip_prefix("0x")
        .or_else(|| digits.strip_prefix("0X"))
    {
        return !hex.is_empty() && hex.chars().all(|c| c.is_ascii_hexdigit());
    }
    if let Some(oct) = digits
        .strip_prefix("0o")
        .or_else(|| digits.strip_prefix("0O"))
    {
        return !oct.is_empty() && oct.chars().all(|c| ('0'..='7').contains(&c));
    }
    if let Some(bin) = digits
        .strip_prefix("0b")
        .or_else(|| digits.strip_prefix("0B"))
    {
        return !bin.is_empty() && bin.chars().all(|c| matches!(c, '0' | '1'));
    }
    let (mantissa, exponent) = match digits.split_once(['e', 'E']) {
        Some((m, e)) => (m, Some(e)),
        None => (digits, None),
    };
    if let Some(exponent) = exponent {
        let exponent = exponent.strip_prefix(['+', '-']).unwrap_or(exponent);
        if exponent.is_empty() || !exponent.chars().all(|c| c.is_ascii_digit()) {
            return false;
        }
    }
    let (whole, fraction) = match mantissa.split_once('.') {
        Some((w, f)) => (w, Some(f)),
        None => (mantissa, None),
    };
    let whole_ok = whole.chars().all(|c| c.is_ascii_digit());
    let fraction_ok = fraction.is_none_or(|f| f.chars().all(|c| c.is_ascii_digit()));
    whole_ok && fraction_ok && (!whole.is_empty() || !fraction.unwrap_or("").is_empty())
}

/// JavaScript identifier start: a letter, `_`, `$`, or any non-ASCII character
/// (the reference accepts `é` and `日本` as segment names).
fn is_identifier_start(c: char) -> bool {
    c.is_alphabetic() || c == '_' || c == '$' || !c.is_ascii()
}

/// JavaScript identifier continuation: a start character or a digit.
fn is_identifier_continue(c: char) -> bool {
    is_identifier_start(c) || c.is_ascii_digit()
}

/// Whether a character is one of the three quote characters the reference
/// strips from a path before it splits it.
fn is_quote(c: char) -> bool {
    matches!(c, '"' | '\'' | '`')
}
