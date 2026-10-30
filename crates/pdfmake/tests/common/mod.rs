//! A zlib inflater for the tests, so the crate keeps no dependency beyond the
//! standard library.
//!
//! `pdfmake` compresses its content streams and the tests must decompress them
//! again to check the text survived. The obvious way to do that is to depend on
//! another crate's inflater, but the workspace allows only `xmlkit` as a
//! cross-crate dependency, and a test dependency is still a dependency. This is
//! a small inflater of the test's own instead.
//!
//! It is not taken on trust: `tests/zlib.rs` checks it against streams Python's
//! `zlib` produced, which is the reference and never saw this code. Stored and
//! fixed-Huffman blocks are both covered there, which is what the crate writes
//! and what a `FlateDecode` stream from another producer may hold.
//!
//! Only what a decoder needs is here. The encoder in `src/flate.rs` writes
//! fixed-Huffman blocks; this reads stored and fixed-Huffman blocks and rejects
//! a dynamic-Huffman block rather than pretending to understand it.

/// Inflates a zlib stream, verifying its Adler-32 trailer.
///
/// Returns `None` for a stream that is malformed, truncated, uses dynamic
/// Huffman, or whose checksum does not match.
#[must_use]
pub fn inflate(data: &[u8]) -> Option<Vec<u8>> {
    let (header, body) = data.split_first_chunk::<2>()?;
    // CM must be 8 (deflate), the header must be a multiple of 31, and FDICT
    // (a preset dictionary) must be clear.
    if header[0] & 0x0f != 8 || header[1] & 0x20 != 0 {
        return None;
    }
    if (u16::from(header[0]) * 256 + u16::from(header[1])) % 31 != 0 {
        return None;
    }
    let (deflate, trailer) = body.split_at(body.len().checked_sub(4)?);
    let out = inflate_raw(deflate)?;
    let expected = u32::from_be_bytes(trailer.try_into().ok()?);
    (adler32(&out) == expected).then_some(out)
}

/// Inflates a raw DEFLATE stream, with no zlib header or checksum.
fn inflate_raw(data: &[u8]) -> Option<Vec<u8>> {
    let mut bits = Bits::new(data);
    let mut out = Vec::new();
    loop {
        let last = bits.take(1)? == 1;
        match bits.take(2)? {
            0 => stored_block(&mut bits, &mut out)?,
            // Fixed Huffman. Dynamic Huffman (BTYPE 2) and the reserved BTYPE 3
            // are not produced by this crate, so they are rejected rather than
            // decoded wrongly.
            1 => fixed_block(&mut bits, &mut out)?,
            _ => return None,
        }
        if last {
            return Some(out);
        }
    }
}

/// A stored block: a length, its complement, then that many literal bytes.
fn stored_block(bits: &mut Bits<'_>, out: &mut Vec<u8>) -> Option<()> {
    bits.align();
    let length = bits.take(16)? as usize;
    let complement = bits.take(16)? as usize;
    if complement != (!length & 0xffff) {
        return None;
    }
    out.extend_from_slice(bits.bytes(length)?);
    Some(())
}

/// Decodes a fixed-Huffman block until its end-of-block symbol.
fn fixed_block(bits: &mut Bits<'_>, out: &mut Vec<u8>) -> Option<()> {
    loop {
        let symbol = fixed_literals().decode(bits)?;
        match symbol {
            0..=255 => out.push(symbol as u8),
            256 => return Some(()),
            257..=285 => {
                let index = symbol as usize - 257;
                let length = LENGTH_BASE[index] as usize + bits.take(LENGTH_EXTRA[index])? as usize;
                let distance_symbol = fixed_distances().decode(bits)? as usize;
                if distance_symbol >= DISTANCE_BASE.len() {
                    return None;
                }
                let distance = DISTANCE_BASE[distance_symbol] as usize
                    + bits.take(DISTANCE_EXTRA[distance_symbol])? as usize;
                if distance == 0 || distance > out.len() {
                    return None;
                }
                let start = out.len() - distance;
                for offset in 0..length {
                    // Byte by byte: the source may overlap the destination,
                    // which is how a run is encoded, so it must not be a memcpy.
                    let byte = out[start + offset];
                    out.push(byte);
                }
            }
            // 286 and 287 are not valid lengths.
            _ => return None,
        }
    }
}

/// The fixed literal/length code of RFC 1951, as a canonical Huffman code.
fn fixed_literals() -> Huffman {
    let mut lengths = [0u8; 288];
    for (symbol, length) in lengths.iter_mut().enumerate() {
        *length = match symbol {
            0..=143 => 8,
            144..=255 => 9,
            256..=279 => 7,
            _ => 8,
        };
    }
    Huffman::from_lengths(&lengths).expect("the fixed literal table is valid")
}

/// The fixed distance code: thirty codes, all five bits.
fn fixed_distances() -> Huffman {
    Huffman::from_lengths(&[5u8; 30]).expect("the fixed distance table is valid")
}

/// Base lengths for the length codes 257 to 285.
const LENGTH_BASE: [u16; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
/// Extra bits for each length code.
const LENGTH_EXTRA: [u32; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
/// Base distances for the distance codes.
const DISTANCE_BASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
/// Extra bits for each distance code.
const DISTANCE_EXTRA: [u32; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];

/// A canonical Huffman code, decoded one bit at a time.
///
/// Walking the code lengths needs no allocation proportional to a lookup table
/// and the code is short and easy to check, which is what a test helper wants.
struct Huffman {
    /// `counts[length]` is how many codes have that length.
    counts: [u16; 16],
    /// Symbols ordered by code length, then by symbol.
    symbols: Vec<u16>,
}

impl Huffman {
    /// Builds a code from the length of each symbol's code, zero meaning unused.
    fn from_lengths(lengths: &[u8]) -> Option<Self> {
        let mut counts = [0u16; 16];
        for &length in lengths {
            if length > 15 {
                return None;
            }
            counts[length as usize] += 1;
        }
        // The first entry is the count of unused symbols, which are not codes.
        counts[0] = 0;

        let mut offsets = [0u16; 16];
        for length in 1..15 {
            offsets[length + 1] = offsets[length] + counts[length];
        }
        let mut symbols = vec![0u16; lengths.iter().filter(|&&length| length != 0).count()];
        for (symbol, &length) in lengths.iter().enumerate() {
            if length != 0 {
                symbols[offsets[length as usize] as usize] = symbol as u16;
                offsets[length as usize] += 1;
            }
        }
        Some(Self { counts, symbols })
    }

    /// Reads one symbol.
    fn decode(&self, bits: &mut Bits<'_>) -> Option<u16> {
        let mut code = 0i32;
        let mut first = 0i32;
        let mut index = 0i32;
        for length in 1..=15usize {
            code |= i32::from(bits.take(1)? as u8);
            let count = i32::from(self.counts[length]);
            if code - first < count {
                return Some(self.symbols[(index + (code - first)) as usize]);
            }
            index += count;
            first = (first + count) << 1;
            code <<= 1;
        }
        None
    }
}

/// Reads bits least-significant first, which is the order DEFLATE defines.
struct Bits<'a> {
    data: &'a [u8],
    position: usize,
    buffer: u32,
    available: u32,
}

impl<'a> Bits<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            position: 0,
            buffer: 0,
            available: 0,
        }
    }

    /// Reads `count` bits, or `None` at the end of the input.
    fn take(&mut self, count: u32) -> Option<u32> {
        if count == 0 {
            return Some(0);
        }
        while self.available < count {
            let byte = *self.data.get(self.position)?;
            self.position += 1;
            self.buffer |= u32::from(byte) << self.available;
            self.available += 8;
        }
        let value = self.buffer & ((1 << count) - 1);
        self.buffer >>= count;
        self.available -= count;
        Some(value)
    }

    /// Discards bits up to the next byte boundary.
    fn align(&mut self) {
        let drop = self.available % 8;
        self.buffer >>= drop;
        self.available -= drop;
    }

    /// Reads whole bytes, which a stored block's payload is.
    fn bytes(&mut self, count: usize) -> Option<&'a [u8]> {
        self.align();
        // Anything buffered is whole bytes now, so return it to the stream.
        let buffered = (self.available / 8) as usize;
        self.position -= buffered;
        self.buffer = 0;
        self.available = 0;
        let end = self.position.checked_add(count)?;
        let slice = self.data.get(self.position..end)?;
        self.position = end;
        Some(slice)
    }
}

/// The Adler-32 checksum zlib requires.
fn adler32(input: &[u8]) -> u32 {
    const MODULUS: u32 = 65521;
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    for &byte in input {
        a = (a + u32::from(byte)) % MODULUS;
        b = (b + a) % MODULUS;
    }
    (b << 16) | a
}
