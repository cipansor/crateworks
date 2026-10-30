//! Checks the test inflater against Python's `zlib`.
//!
//! `tests/common/mod.rs` carries an inflater so the crate needs no dependency
//! beyond the standard library. An inflater checked only against the encoder it
//! is meant to verify proves nothing, so it is checked here against streams
//! Python's `zlib` produced - the reference, which never saw this code. Between
//! them the vectors cover stored and fixed-Huffman blocks, which is what a
//! `FlateDecode` stream may hold.

mod common;

/// Decodes a hex string.
fn unhex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&text[index..index + 2], 16).unwrap())
        .collect()
}

/// zlib streams paired with the plain bytes they hold, produced by `zlib`.
///
/// Rebuilt with `scripts/fuzz/pdfmake/gen_zlib_vectors.py`.
const ZLIB_VECTORS: &str = include_str!("fixtures/zlib_vectors.txt");

#[test]
fn the_test_inflater_matches_zlib_on_every_vector() {
    let mut checked = 0;
    for (number, line) in ZLIB_VECTORS.lines().enumerate() {
        let (compressed, plain) = line.split_once('\t').expect("hex\thex");
        let compressed = unhex(compressed);
        let plain = unhex(plain);
        assert_eq!(
            common::inflate(&compressed).as_deref(),
            Some(plain.as_slice()),
            "vector {number} ({} compressed bytes)",
            compressed.len()
        );
        checked += 1;
    }
    assert!(checked > 5, "only {checked} vectors were read");
}
