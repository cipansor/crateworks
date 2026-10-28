#![doc = include_str!("../README.md")]
mod escape;
mod reader;
mod time;
mod writer;
mod xml;

pub use escape::{Error, unescape};
pub use reader::{Properties, load, parse, parse_str};
pub use time::java_timestamp_utc;
pub use writer::{
    WriteOptions, escape, escape_key, escape_value, join_key_value, store, to_comment,
};
pub use xml::{dumps_xml, loads_xml};
