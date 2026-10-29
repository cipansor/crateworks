# fast-redact

[![crates.io](https://img.shields.io/crates/v/fast-redact.svg)](https://crates.io/crates/fast-redact)
[![docs.rs](https://docs.rs/fast-redact/badge.svg)](https://docs.rs/fast-redact)

Redact secrets from a JSON document before it is logged.

A log line is usually a JSON document, and the fields that must never be written
to disk — a cookie, an `Authorization` header, a password — sit at known
locations in it. This crate takes those locations as paths and returns the
document with the values at them replaced, so a request can be logged whole
without a secret in it.

It is a port of the npm package [`fast-redact`][fast-redact], which is what the
`pino` logger uses. The behaviour matches the reference and is checked against
it case by case.

[fast-redact]: https://github.com/davidmarkclements/fast-redact

## Example

```rust
use fast_redact::{Redactor, Value};

let document = Value::parse(
    r#"{"headers":{"host":"example.com","cookie":"session=abc"}}"#,
).unwrap();

let redactor = Redactor::new(["headers.cookie"]).unwrap();

assert_eq!(
    redactor.redact(&document),
    r#"{"headers":{"host":"example.com","cookie":"[REDACTED]"}}"#,
);
// The input document is left alone.
assert_eq!(document.to_json(), r#"{"headers":{"host":"example.com","cookie":"session=abc"}}"#);
```

## Paths

A path names a location in the document:

| Path | Matches |
|---|---|
| `headers.cookie` | the `cookie` key of `headers` |
| `headers["x-forwarded-for"]` | a key that is not an identifier |
| `items[0].secret` | the first element of an array |
| `users.*.token` | `token` of every user |
| `headers.*` | every key of `headers` |

A path is validated when the redactor is built, so a typo is an error rather
than a redaction that silently does nothing.

```rust
use fast_redact::Redactor;

assert!(Redactor::new(["a.b.c"]).is_ok());
assert!(Redactor::new(["a..c"]).is_err());   // an empty segment
assert!(Redactor::new(["a[b]"]).is_err());   // a bare word in a bracket
```

## What replaces a value

By default a redacted value becomes the string `"[REDACTED]"`. The reference's
three alternatives are all available.

```rust
use fast_redact::{Redactor, Value};

let document = Value::parse(r#"{"user":"alice","token":"abcd1234","note":"ok"}"#).unwrap();

// A different fixed value.
let numbered = Redactor::new(["token"]).unwrap().with_censor(Value::Number(0.0));
assert_eq!(numbered.redact(&document), r#"{"user":"alice","token":0,"note":"ok"}"#);

// Remove the key entirely, rather than replacing it.
let removed = Redactor::new(["token"]).unwrap().with_remove();
assert_eq!(removed.redact(&document), r#"{"user":"alice","note":"ok"}"#);

// Compute the replacement from the value and its path.
let shaped = Redactor::new(["token"]).unwrap().with_censor_fn(|value, path| {
    let len = value.as_str().map_or(0, |s| s.chars().count());
    Value::String(format!("{}:<{len} chars>", path.join(".")))
});
assert_eq!(shaped.redact(&document), r#"{"user":"alice","token":"token:<8 chars>","note":"ok"}"#);
```

## Arrays

A wildcard over an array redacts every element; an index redacts one. Removing
an array element sets it to `null`, which is what `JSON.stringify` does with the
`undefined` the reference leaves behind.

```rust
use fast_redact::{Redactor, Value};

let document = Value::parse(r#"{"tokens":["a","b","c"]}"#).unwrap();

let all = Redactor::new(["tokens[*]"]).unwrap();
assert_eq!(all.redact(&document), r#"{"tokens":["[REDACTED]","[REDACTED]","[REDACTED]"]}"#);

let one = Redactor::new(["tokens[1]"]).unwrap();
assert_eq!(one.redact(&document), r#"{"tokens":["a","[REDACTED]","c"]}"#);

let dropped = Redactor::new(["tokens[1]"]).unwrap().with_remove();
assert_eq!(dropped.redact(&document), r#"{"tokens":["a",null,"c"]}"#);
```

## The wildcard rule

A path is split at its **first** `*`: the part before it is the location to
reach, and the part after it is walked from every key there. `users.*.token`
reaches `users`, then takes `token` from each of its values. This is the
reference's rule, and it is why `*.*` behaves the way it does rather than as a
general glob.

## What this crate is

An independent Rust implementation with no dependencies. It carries its own JSON
reader and writer, because the output has to be byte-for-byte what
`JSON.stringify` produces, including the number formatting (`1e21`, not
`1000000000000000000000`) and the escape set. It reads a document, redacts a
copy, and writes the copy; the input is never modified, so unlike the reference
there is no `restore` step and none is needed.
