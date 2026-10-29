//! A JSON value, a parser, and a serializer that agrees with `JSON.stringify`.
//!
//! The redactor works on a parsed document rather than on text, so it needs a
//! JSON model. `serde_json` is not available — this workspace takes no
//! dependencies — so the model, the parser and the writer live here.
//!
//! The writer is the part that has to match another implementation exactly: the
//! point of the crate is to produce the same line the reference produces, so the
//! output has to be byte-for-byte the same as `JSON.stringify`, including the
//! number formatting and the escape set. Both are specified in ECMA-262 and are
//! implemented here from the specification rather than from Rust's `Display`,
//! which disagrees with JavaScript about where the decimal point goes.

use std::fmt::Write as _;

/// A parsed JSON value.
///
/// Objects keep their keys in the order they appeared, because `JSON.stringify`
/// writes them in insertion order and the output of this crate has to match it.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// `null`.
    Null,
    /// `true` or `false`.
    Bool(bool),
    /// A number. JSON has one numeric type; it is held as `f64`, which is what
    /// JavaScript uses, so that a value survives a parse and a write unchanged.
    Number(f64),
    /// A string.
    String(String),
    /// An array, in order.
    Array(Vec<Value>),
    /// An object, as key/value pairs in insertion order.
    Object(Vec<(String, Value)>),
}

/// A parse failure, with the byte offset where it was noticed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// Byte offset into the input where the parse failed.
    pub offset: usize,
    /// What was wrong.
    pub message: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid JSON at byte {}: {}", self.offset, self.message)
    }
}

impl std::error::Error for ParseError {}

impl Value {
    /// Parse a JSON document.
    ///
    /// Leading and trailing whitespace is allowed; trailing content after the
    /// first value is not.
    ///
    /// ```
    /// use fast_redact::Value;
    /// let value = Value::parse(r#"{"a": [1, true, null]}"#).unwrap();
    /// assert_eq!(value.to_json(), r#"{"a":[1,true,null]}"#);
    /// ```
    pub fn parse(input: &str) -> Result<Value, ParseError> {
        let mut parser = Parser {
            bytes: input.as_bytes(),
            pos: 0,
        };
        parser.skip_whitespace();
        let value = parser.value()?;
        parser.skip_whitespace();
        if parser.pos != parser.bytes.len() {
            return Err(parser.error("trailing characters after the JSON value"));
        }
        Ok(value)
    }

    /// Write the value the way `JSON.stringify` writes it.
    ///
    /// ```
    /// use fast_redact::Value;
    /// // JavaScript prints 1e21 with an exponent and 1e-7 likewise.
    /// assert_eq!(Value::Number(1e21).to_json(), "1e+21");
    /// assert_eq!(Value::Number(1e-7).to_json(), "1e-7");
    /// ```
    #[must_use]
    pub fn to_json(&self) -> String {
        let mut out = String::new();
        self.write(&mut out);
        out
    }

    /// The value as an object's pairs, or `None` for any other kind.
    #[must_use]
    pub fn as_object(&self) -> Option<&[(String, Value)]> {
        match self {
            Value::Object(pairs) => Some(pairs),
            _ => None,
        }
    }

    /// The value as an array's elements, or `None` for any other kind.
    #[must_use]
    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Value::Array(items) => Some(items),
            _ => None,
        }
    }

    /// The value as a string, or `None` for any other kind.
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(text) => Some(text),
            _ => None,
        }
    }

    /// The value as a number, or `None` for any other kind.
    #[must_use]
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Number(number) => Some(*number),
            _ => None,
        }
    }

    /// Whether the value is `null`.
    #[must_use]
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    fn write(&self, out: &mut String) {
        match self {
            Value::Null => out.push_str("null"),
            Value::Bool(true) => out.push_str("true"),
            Value::Bool(false) => out.push_str("false"),
            Value::Number(number) => write_number(*number, out),
            Value::String(text) => write_string(text, out),
            Value::Array(items) => {
                out.push('[');
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    item.write(out);
                }
                out.push(']');
            }
            Value::Object(pairs) => {
                out.push('{');
                for (index, (key, value)) in pairs.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    write_string(key, out);
                    out.push(':');
                    value.write(out);
                }
                out.push('}');
            }
        }
    }
}

/// Write a string the way `JSON.stringify` writes it.
///
/// The escape set is fixed: the quote, the backslash, the five named controls,
/// and every other code point below `U+0020` as `\u00xx`. Everything else is
/// written as itself — `JSON.stringify` does not escape `/`, and it emits the
/// non-ASCII characters literally.
fn write_string(text: &str, out: &mut String) {
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{08}' => out.push_str("\\b"),
            '\u{09}' => out.push_str("\\t"),
            '\u{0a}' => out.push_str("\\n"),
            '\u{0c}' => out.push_str("\\f"),
            '\u{0d}' => out.push_str("\\r"),
            ch if (ch as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", ch as u32);
            }
            ch => out.push(ch),
        }
    }
    out.push('"');
}

/// Numbers, written the way JavaScript writes them.
///
/// Rust and JavaScript both print the shortest text that reads back as the same
/// number, but they disagree about *where* the decimal point goes: for `1e21`
/// Rust prints twenty-two digits and JavaScript prints `1e+21`, and for `1e-7`
/// Rust prints `0.0000001` and JavaScript prints `1e-7`. The rules from
/// ECMA-262 §6.1.6.1.20 are followed directly rather than Rust's `Display`.
fn write_number(number: f64, out: &mut String) {
    // JSON has no way to write these; `JSON.stringify` turns each into `null`.
    if number.is_nan() || number.is_infinite() {
        out.push_str("null");
        return;
    }
    if number == 0.0 {
        // Covers -0.0: JavaScript prints it as "0".
        out.push('0');
        return;
    }

    let sign = if number < 0.0 { "-" } else { "" };
    // `{:e}` gives the shortest digits that read back as the same number, in the
    // form `1.23456e7`, which is the one piece both languages agree on.
    let scientific = format!("{:e}", number.abs());
    let (mantissa, exponent) = scientific.split_once('e').expect("scientific form");
    let exponent: i32 = exponent.parse().expect("exponent");

    let mut digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    while digits.len() > 1 && digits.ends_with('0') {
        digits.pop();
    }

    let digits = digits.as_str();
    let count = digits.len() as i32;
    // Where the decimal point sits, as JavaScript counts it.
    let point = exponent + 1;

    if count <= point && point <= 21 {
        let mut text = String::from(digits);
        text.extend(std::iter::repeat_n('0', (point - count) as usize));
        out.push_str(sign);
        out.push_str(&text);
    } else if 0 < point && point <= 21 {
        let split = point as usize;
        out.push_str(sign);
        out.push_str(&digits[..split]);
        out.push('.');
        out.push_str(&digits[split..]);
    } else if -6 < point && point <= 0 {
        out.push_str(sign);
        out.push_str("0.");
        out.extend(std::iter::repeat_n('0', (-point) as usize));
        out.push_str(digits);
    } else if count == 1 {
        let _ = write!(
            out,
            "{sign}{digits}e{}{}",
            exponent_sign(point - 1),
            (point - 1).abs()
        );
    } else {
        let _ = write!(
            out,
            "{sign}{}.{}e{}{}",
            &digits[..1],
            &digits[1..],
            exponent_sign(point - 1),
            (point - 1).abs()
        );
    }
}

/// JavaScript writes a plus sign on a non-negative exponent.
fn exponent_sign(exponent: i32) -> char {
    if exponent < 0 { '-' } else { '+' }
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl Parser<'_> {
    fn error(&self, message: &str) -> ParseError {
        ParseError {
            offset: self.pos,
            message: message.to_string(),
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn skip_whitespace(&mut self) {
        while let Some(byte) = self.peek() {
            if matches!(byte, b' ' | b'\t' | b'\n' | b'\r') {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn expect(&mut self, byte: u8, message: &str) -> Result<(), ParseError> {
        if self.peek() == Some(byte) {
            self.pos += 1;
            Ok(())
        } else {
            Err(self.error(message))
        }
    }

    fn literal(&mut self, text: &str, value: Value) -> Result<Value, ParseError> {
        if self.bytes[self.pos..].starts_with(text.as_bytes()) {
            self.pos += text.len();
            Ok(value)
        } else {
            Err(self.error("expected a JSON value"))
        }
    }

    fn value(&mut self) -> Result<Value, ParseError> {
        match self.peek() {
            Some(b'{') => self.object(),
            Some(b'[') => self.array(),
            Some(b'"') => Ok(Value::String(self.string()?)),
            Some(b't') => self.literal("true", Value::Bool(true)),
            Some(b'f') => self.literal("false", Value::Bool(false)),
            Some(b'n') => self.literal("null", Value::Null),
            Some(b'-' | b'0'..=b'9') => self.number(),
            _ => Err(self.error("expected a JSON value")),
        }
    }

    fn object(&mut self) -> Result<Value, ParseError> {
        self.expect(b'{', "expected `{`")?;
        let mut pairs = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(Value::Object(pairs));
        }
        loop {
            self.skip_whitespace();
            if self.peek() != Some(b'"') {
                return Err(self.error("expected a string key"));
            }
            let key = self.string()?;
            self.skip_whitespace();
            self.expect(b':', "expected `:` after the key")?;
            self.skip_whitespace();
            let value = self.value()?;
            pairs.push((key, value));
            self.skip_whitespace();
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                }
                Some(b'}') => {
                    self.pos += 1;
                    return Ok(Value::Object(pairs));
                }
                _ => return Err(self.error("expected `,` or `}`")),
            }
        }
    }

    fn array(&mut self) -> Result<Value, ParseError> {
        self.expect(b'[', "expected `[`")?;
        let mut items = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(Value::Array(items));
        }
        loop {
            self.skip_whitespace();
            items.push(self.value()?);
            self.skip_whitespace();
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                }
                Some(b']') => {
                    self.pos += 1;
                    return Ok(Value::Array(items));
                }
                _ => return Err(self.error("expected `,` or `]`")),
            }
        }
    }

    fn string(&mut self) -> Result<String, ParseError> {
        self.expect(b'"', "expected `\"`")?;
        let mut out = String::new();
        loop {
            let byte = self
                .peek()
                .ok_or_else(|| self.error("unterminated string"))?;
            match byte {
                b'"' => {
                    self.pos += 1;
                    return Ok(out);
                }
                b'\\' => {
                    self.pos += 1;
                    self.escape(&mut out)?;
                }
                byte if byte < 0x20 => {
                    return Err(self.error("unescaped control character in string"));
                }
                _ => {
                    let start = self.pos;
                    // Copy one whole UTF-8 character.
                    let text = std::str::from_utf8(&self.bytes[start..])
                        .map_err(|_| self.error("invalid UTF-8 in string"))?;
                    let ch = text.chars().next().expect("a character");
                    out.push(ch);
                    self.pos += ch.len_utf8();
                }
            }
        }
    }

    fn escape(&mut self, out: &mut String) -> Result<(), ParseError> {
        let byte = self
            .peek()
            .ok_or_else(|| self.error("unterminated escape"))?;
        self.pos += 1;
        match byte {
            b'"' => out.push('"'),
            b'\\' => out.push('\\'),
            b'/' => out.push('/'),
            b'b' => out.push('\u{08}'),
            b'f' => out.push('\u{0c}'),
            b'n' => out.push('\n'),
            b'r' => out.push('\r'),
            b't' => out.push('\t'),
            b'u' => {
                let code = self.hex4()?;
                // A high surrogate must be followed by a low surrogate; a lone
                // surrogate is not a character and cannot be stored in a Rust
                // string, so it becomes the replacement character.
                if (0xd800..0xdc00).contains(&code) {
                    if self.peek() == Some(b'\\') && self.bytes.get(self.pos + 1) == Some(&b'u') {
                        self.pos += 2;
                        let low = self.hex4()?;
                        if (0xdc00..0xe000).contains(&low) {
                            let combined = 0x10000 + ((code - 0xd800) << 10) + (low - 0xdc00);
                            out.push(char::from_u32(combined).unwrap_or('\u{fffd}'));
                            return Ok(());
                        }
                    }
                    out.push('\u{fffd}');
                } else if (0xdc00..0xe000).contains(&code) {
                    out.push('\u{fffd}');
                } else {
                    out.push(char::from_u32(code).unwrap_or('\u{fffd}'));
                }
            }
            _ => return Err(self.error("invalid escape")),
        }
        Ok(())
    }

    fn hex4(&mut self) -> Result<u32, ParseError> {
        let mut value = 0u32;
        for _ in 0..4 {
            let byte = self.peek().ok_or_else(|| self.error("short \\u escape"))?;
            let digit = (byte as char)
                .to_digit(16)
                .ok_or_else(|| self.error("invalid \\u escape"))?;
            value = value * 16 + digit;
            self.pos += 1;
        }
        Ok(value)
    }

    fn number(&mut self) -> Result<Value, ParseError> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }
        // Integer part.
        match self.peek() {
            Some(b'0') => self.pos += 1,
            Some(b'1'..=b'9') => {
                while matches!(self.peek(), Some(b'0'..=b'9')) {
                    self.pos += 1;
                }
            }
            _ => return Err(self.error("expected a digit")),
        }
        // Fraction.
        if self.peek() == Some(b'.') {
            self.pos += 1;
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(self.error("expected a digit after the decimal point"));
            }
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.pos += 1;
            }
        }
        // Exponent.
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.pos += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.pos += 1;
            }
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(self.error("expected a digit in the exponent"));
            }
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.pos += 1;
            }
        }
        let text = std::str::from_utf8(&self.bytes[start..self.pos]).expect("ASCII digits");
        text.parse::<f64>()
            .map(Value::Number)
            .map_err(|_| ParseError {
                offset: start,
                message: "number out of range".to_string(),
            })
    }
}
