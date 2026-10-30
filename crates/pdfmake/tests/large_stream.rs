//! A content stream longer than the DEFLATE hash table must still compress.
//!
//! The matcher keeps one chain entry per input position. That array was sized by
//! the hash table (32768) instead of by the input, so once a stream grew past
//! 32768 bytes the walk off a hash bucket indexed past the end of the array and
//! panicked. A document of a few hundred paragraphs produces a stream that long,
//! so the crash is reachable with ordinary input.
//!
//! The fix is only half the check: the compressed stream is decompressed again
//! with the inflater in the `pdfa` crate, which is an independent implementation,
//! so the test proves the output is still a stream a reader can decode rather
//! than merely that the panic is gone.

use pdfmake::{Document, Paragraph};

/// The bytes of every `FlateDecode` stream in a PDF, between `stream` and
/// `endstream`.
fn stream_payloads(pdf: &[u8]) -> Vec<&[u8]> {
    const OPEN: &[u8] = b"\nstream\n";
    const CLOSE: &[u8] = b"\nendstream";
    let mut payloads = Vec::new();
    let mut from = 0;
    while let Some(open) = find(pdf, OPEN, from) {
        let start = open + OPEN.len();
        let end = find(pdf, CLOSE, start).expect("every stream is closed");
        payloads.push(&pdf[start..end]);
        from = end;
    }
    payloads
}

fn find(haystack: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    haystack[from..]
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|offset| from + offset)
}

#[test]
fn a_content_stream_past_the_hash_table_size_still_compresses() {
    let mut document = Document::new();
    for index in 0..800 {
        document = document.push(Paragraph::new(format!(
            "line number {index} with some words"
        )));
    }

    let pdf = document.to_pdf();
    let payloads = stream_payloads(&pdf);
    assert!(!payloads.is_empty(), "the document has no content stream");

    let mut decoded = Vec::new();
    for payload in payloads {
        let inflated = pdfa::flate::inflate(payload).expect("a FlateDecode stream must inflate");
        decoded.extend_from_slice(&inflated);
    }

    // The bug only appears once a stream is longer than the hash table, so the
    // test is only meaningful if the input reaches that size.
    assert!(
        decoded.len() > 32 * 1024,
        "the decoded content is only {} bytes, below the 32 KiB that triggered the bug",
        decoded.len()
    );

    let text = String::from_utf8_lossy(&decoded);
    assert!(text.contains("line number 0 with some words"));
    assert!(text.contains("line number 799 with some words"));
}
