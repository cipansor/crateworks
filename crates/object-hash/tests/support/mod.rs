//! A reader for the tagged value descriptions the fixtures carry.
//!
//! Shared by the fuzz and objects suites. The format is a small tagged JSON:
//! `{"n":..}` numbers, `{"a":..}` arrays, `{"s":..}` sets, `{"m":..}` maps and
//! `{"o":..}` plain objects, parsed here rather than pulled in as a dependency,
//! since this crate has none.

use object_hash::Value;

pub struct Reader<'a> {
    bytes: &'a [u8],
    pub at: usize,
}

impl<'a> Reader<'a> {
    pub fn new(text: &'a str) -> Self {
        Self {
            bytes: text.as_bytes(),
            at: 0,
        }
    }

    fn peek(&self) -> u8 {
        self.bytes[self.at]
    }

    fn take(&mut self, expected: u8) {
        assert_eq!(
            self.bytes[self.at], expected,
            "expected {:?} at byte {}",
            expected as char, self.at
        );
        self.at += 1;
    }

    fn literal(&mut self, text: &str) {
        assert!(
            self.bytes[self.at..].starts_with(text.as_bytes()),
            "expected {text:?} at byte {}",
            self.at
        );
        self.at += text.len();
    }

    pub fn value(&mut self) -> Value {
        match self.peek() {
            b'n' if self.bytes[self.at..].starts_with(b"null") => {
                self.at += 4;
                Value::Null
            }
            b't' => {
                self.literal("true");
                Value::Bool(true)
            }
            b'f' => {
                self.literal("false");
                Value::Bool(false)
            }
            b'"' => Value::String(self.string()),
            b'{' => self.tagged(),
            other => panic!("unexpected {:?} at byte {}", other as char, self.at),
        }
    }

    /// One of `{"n":..}`, `{"a":[..]}`, `{"s":[..]}` or `{"m":[[key,value],..]}`.
    fn tagged(&mut self) -> Value {
        self.take(b'{');
        self.take(b'"');
        let tag = self.peek();
        self.at += 1;
        self.take(b'"');
        self.take(b':');
        let value = match tag {
            b'n' => {
                let text = self.string();
                Value::Number(match text.as_str() {
                    "NaN" => f64::NAN,
                    "Infinity" => f64::INFINITY,
                    "-Infinity" => f64::NEG_INFINITY,
                    _ => text.parse().unwrap_or_else(|_| panic!("number {text:?}")),
                })
            }
            b'a' => Value::Array(self.list()),
            b's' => Value::Set(self.list()),
            b'm' => Value::Map(self.pairs()),
            b'o' => Value::Object(self.pairs()),
            other => panic!("tag {:?}", other as char),
        };
        self.take(b'}');
        value
    }

    fn list(&mut self) -> Vec<Value> {
        self.take(b'[');
        let mut items = Vec::new();
        if self.peek() == b']' {
            self.at += 1;
            return items;
        }
        loop {
            items.push(self.value());
            match self.peek() {
                b',' => self.at += 1,
                _ => {
                    self.take(b']');
                    return items;
                }
            }
        }
    }

    fn pairs(&mut self) -> Vec<(String, Value)> {
        self.take(b'[');
        let mut pairs = Vec::new();
        if self.peek() == b']' {
            self.at += 1;
            return pairs;
        }
        loop {
            self.take(b'[');
            let key = self.string();
            self.take(b',');
            let value = self.value();
            self.take(b']');
            pairs.push((key, value));
            match self.peek() {
                b',' => self.at += 1,
                _ => {
                    self.take(b']');
                    return pairs;
                }
            }
        }
    }

    fn string(&mut self) -> String {
        self.take(b'"');
        let mut text = String::new();
        loop {
            match self.peek() {
                b'"' => {
                    self.at += 1;
                    return text;
                }
                b'\\' => {
                    self.at += 1;
                    let escape = self.peek();
                    self.at += 1;
                    match escape {
                        b'u' => {
                            let first = self.hex4();
                            // A lead surrogate is always followed by its trail.
                            let code = if (0xD800..0xDC00).contains(&first) {
                                self.take(b'\\');
                                self.take(b'u');
                                let second = self.hex4();
                                0x1_0000 + ((first - 0xD800) << 10) + (second - 0xDC00)
                            } else {
                                first
                            };
                            text.push(char::from_u32(code).expect("scalar value"));
                        }
                        b'n' => text.push('\n'),
                        b't' => text.push('\t'),
                        b'r' => text.push('\r'),
                        b'"' => text.push('"'),
                        b'\\' => text.push('\\'),
                        other => panic!("escape {:?}", other as char),
                    }
                }
                _ => {
                    // Copy the run of bytes up to the next quote or backslash.
                    let start = self.at;
                    while !matches!(self.peek(), b'"' | b'\\') {
                        self.at += 1;
                    }
                    text.push_str(std::str::from_utf8(&self.bytes[start..self.at]).expect("utf-8"));
                }
            }
        }
    }

    fn hex4(&mut self) -> u32 {
        let digits =
            std::str::from_utf8(&self.bytes[self.at..self.at + 4]).expect("ascii hex digits");
        self.at += 4;
        u32::from_str_radix(digits, 16).expect("hex")
    }
}
