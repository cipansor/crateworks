//! The error type.

use std::fmt;

/// Something that prevented a layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The request was understood but cannot be carried out, such as a page
    /// width that is zero, negative or not finite.
    Unsupported(String),
    /// The input could not be read, such as a CSS declaration with no value.
    Malformed(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported(message) => write!(f, "unsupported: {message}"),
            Self::Malformed(message) => write!(f, "malformed input: {message}"),
        }
    }
}

impl std::error::Error for Error {}
