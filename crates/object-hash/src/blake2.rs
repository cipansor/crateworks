//! BLAKE2b-512 and BLAKE2s-256, unkeyed, for callers whose other system
//! already speaks them.
//!
//! Written out for the same reason as the SHA family: the crate has no
//! dependencies, and the function is short. The reference package offers
//! `blake2b512` and `blake2s256` through Node's `crypto`, and this reproduces
//! their digests for the same canonical bytes.
//!
//! BLAKE2 is a ChaCha-derived ARX construction. It parameterises a hash by
//! digest length, key length, fanout and depth, all packed into one word that
//! is exclusive-ored into the initial state. Only the unkeyed, single-threaded
//! form is here, because that is what a hash of a canonical form needs; a
//! keyed MAC would be a different function with a different name, and the
//! reference's `crypto` digest does not key either.
//!
//! The two widths differ in word size (64 against 32 bits), block size (128
//! against 64 bytes), round count (12 against 10) and rotation constants, so
//! they share the message schedule and nothing else.

/// The BLAKE2b-512 digest of `data`.
pub(crate) fn digest512(data: &[u8]) -> [u8; 64] {
    let mut h = IV64;
    // The parameter block: digest length 64, key length 0, fanout 1, depth 1.
    h[0] ^= 0x0101_0040;

    let mut counter: u128 = 0;
    if data.is_empty() {
        // BLAKE2 always compresses a final block; for empty input it is empty.
        compress64(&mut h, &[], 0, true);
    } else {
        let mut blocks = data.chunks(128).peekable();
        while let Some(block) = blocks.next() {
            counter += block.len() as u128;
            compress64(&mut h, block, counter, blocks.peek().is_none());
        }
    }

    let mut out = [0u8; 64];
    for (chunk, word) in out.as_chunks_mut::<8>().0.iter_mut().zip(h) {
        chunk.copy_from_slice(&word.to_le_bytes());
    }
    out
}

/// The BLAKE2s-256 digest of `data`.
pub(crate) fn digest256(data: &[u8]) -> [u8; 32] {
    let mut h = IV32;
    // The parameter block: digest length 32, key length 0, fanout 1, depth 1.
    h[0] ^= 0x0101_0020;

    let mut counter: u64 = 0;
    if data.is_empty() {
        compress32(&mut h, &[], 0, true);
    } else {
        let mut blocks = data.chunks(64).peekable();
        while let Some(block) = blocks.next() {
            counter += block.len() as u64;
            compress32(&mut h, block, counter, blocks.peek().is_none());
        }
    }

    let mut out = [0u8; 32];
    for (chunk, word) in out.as_chunks_mut::<4>().0.iter_mut().zip(h) {
        chunk.copy_from_slice(&word.to_le_bytes());
    }
    out
}

/// The BLAKE2b round constants, the same as SHA-512's initial state.
const IV64: [u64; 8] = [
    0x6a09_e667_f3bc_c908,
    0xbb67_ae85_84ca_a73b,
    0x3c6e_f372_fe94_f82b,
    0xa54f_f53a_5f1d_36f1,
    0x510e_527f_ade6_82d1,
    0x9b05_688c_2b3e_6c1f,
    0x1f83_d9ab_fb41_bd6b,
    0x5be0_cd19_137e_2179,
];

/// The BLAKE2s round constants, the same as SHA-256's initial state.
const IV32: [u32; 8] = [
    0x6a09_e667,
    0xbb67_ae85,
    0x3c6e_f372,
    0xa54f_f53a,
    0x510e_527f,
    0x9b05_688c,
    0x1f83_d9ab,
    0x5be0_cd19,
];

/// The message schedule, ten rows. BLAKE2b's rounds 10 and 11 reuse rows 0
/// and 1; BLAKE2s stops after row 9.
const SIGMA: [[usize; 16]; 10] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3],
    [11, 8, 12, 0, 5, 2, 15, 13, 10, 14, 3, 6, 7, 1, 9, 4],
    [7, 9, 3, 1, 13, 12, 11, 14, 2, 6, 5, 10, 4, 0, 15, 8],
    [9, 0, 5, 7, 2, 4, 10, 15, 14, 1, 11, 12, 6, 8, 3, 13],
    [2, 12, 6, 10, 0, 11, 8, 3, 4, 13, 7, 5, 15, 14, 1, 9],
    [12, 5, 1, 15, 14, 13, 4, 10, 0, 7, 6, 3, 9, 2, 8, 11],
    [13, 11, 7, 14, 12, 1, 3, 9, 5, 0, 15, 4, 8, 6, 2, 10],
    [6, 15, 14, 9, 11, 3, 0, 8, 12, 2, 13, 7, 1, 4, 10, 5],
    [10, 2, 8, 4, 7, 6, 1, 5, 15, 11, 9, 14, 3, 12, 13, 0],
];

/// One BLAKE2b compression, updating `h` in place.
fn compress64(h: &mut [u64; 8], block: &[u8], counter: u128, last: bool) {
    // A short final block is zero-padded to the full width; reading it with
    // `as_chunks` alone would drop the partial group and zero the real words.
    let mut padded = [0u8; 128];
    padded[..block.len()].copy_from_slice(block);

    let mut m = [0u64; 16];
    for (word, chunk) in m.iter_mut().zip(padded.as_chunks::<8>().0) {
        *word = u64::from_le_bytes([
            chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6], chunk[7],
        ]);
    }

    let mut v = [0u64; 16];
    v[..8].copy_from_slice(h);
    v[8..].copy_from_slice(&IV64);
    v[12] ^= counter as u64;
    v[13] ^= (counter >> 64) as u64;
    if last {
        v[14] = !v[14];
    }

    for round in 0..12 {
        let s = SIGMA[round % 10];
        g64(&mut v, 0, 4, 8, 12, m[s[0]], m[s[1]]);
        g64(&mut v, 1, 5, 9, 13, m[s[2]], m[s[3]]);
        g64(&mut v, 2, 6, 10, 14, m[s[4]], m[s[5]]);
        g64(&mut v, 3, 7, 11, 15, m[s[6]], m[s[7]]);
        g64(&mut v, 0, 5, 10, 15, m[s[8]], m[s[9]]);
        g64(&mut v, 1, 6, 11, 12, m[s[10]], m[s[11]]);
        g64(&mut v, 2, 7, 8, 13, m[s[12]], m[s[13]]);
        g64(&mut v, 3, 4, 9, 14, m[s[14]], m[s[15]]);
    }

    for (slot, index) in h.iter_mut().zip(0..8) {
        *slot ^= v[index] ^ v[index + 8];
    }
}

/// The BLAKE2b mixing function, rotations 32, 24, 16, 63.
fn g64(v: &mut [u64; 16], a: usize, b: usize, c: usize, d: usize, x: u64, y: u64) {
    v[a] = v[a].wrapping_add(v[b]).wrapping_add(x);
    v[d] = (v[d] ^ v[a]).rotate_right(32);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = (v[b] ^ v[c]).rotate_right(24);
    v[a] = v[a].wrapping_add(v[b]).wrapping_add(y);
    v[d] = (v[d] ^ v[a]).rotate_right(16);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = (v[b] ^ v[c]).rotate_right(63);
}

/// One BLAKE2s compression, updating `h` in place.
fn compress32(h: &mut [u32; 8], block: &[u8], counter: u64, last: bool) {
    let mut padded = [0u8; 64];
    padded[..block.len()].copy_from_slice(block);

    let mut m = [0u32; 16];
    for (word, chunk) in m.iter_mut().zip(padded.as_chunks::<4>().0) {
        *word = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
    }

    let mut v = [0u32; 16];
    v[..8].copy_from_slice(h);
    v[8..].copy_from_slice(&IV32);
    v[12] ^= counter as u32;
    v[13] ^= (counter >> 32) as u32;
    if last {
        v[14] = !v[14];
    }

    for s in SIGMA {
        g32(&mut v, 0, 4, 8, 12, m[s[0]], m[s[1]]);
        g32(&mut v, 1, 5, 9, 13, m[s[2]], m[s[3]]);
        g32(&mut v, 2, 6, 10, 14, m[s[4]], m[s[5]]);
        g32(&mut v, 3, 7, 11, 15, m[s[6]], m[s[7]]);
        g32(&mut v, 0, 5, 10, 15, m[s[8]], m[s[9]]);
        g32(&mut v, 1, 6, 11, 12, m[s[10]], m[s[11]]);
        g32(&mut v, 2, 7, 8, 13, m[s[12]], m[s[13]]);
        g32(&mut v, 3, 4, 9, 14, m[s[14]], m[s[15]]);
    }

    for (slot, index) in h.iter_mut().zip(0..8) {
        *slot ^= v[index] ^ v[index + 8];
    }
}

/// The BLAKE2s mixing function, rotations 16, 12, 8, 7.
fn g32(v: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize, x: u32, y: u32) {
    v[a] = v[a].wrapping_add(v[b]).wrapping_add(x);
    v[d] = (v[d] ^ v[a]).rotate_right(16);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = (v[b] ^ v[c]).rotate_right(12);
    v[a] = v[a].wrapping_add(v[b]).wrapping_add(y);
    v[d] = (v[d] ^ v[a]).rotate_right(8);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = (v[b] ^ v[c]).rotate_right(7);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A digest as lowercase hexadecimal, for the published vectors.
    fn hex(bytes: &[u8]) -> String {
        let mut out = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            out.push(char::from_digit(u32::from(byte >> 4), 16).expect("nibble"));
            out.push(char::from_digit(u32::from(byte & 0x0f), 16).expect("nibble"));
        }
        out
    }

    #[test]
    fn the_published_blake2b_vectors_match() {
        // From RFC 7693 appendix A and the BLAKE2 reference vectors.
        assert_eq!(
            hex(&digest512(b"")),
            "786a02f742015903c6c6fd852552d272912f4740e15847618a86e217f71f5419\
             d25e1031afee585313896444934eb04b903a685b1448b755d56f701afe9be2ce"
        );
        assert_eq!(
            hex(&digest512(b"abc")),
            "ba80a53f981c4d0d6a2797b69f12f6e94c212f14685ac4b74b12bb6fdbffa2d1\
             7d87c5392aab792dc252d5de4533cc9518d38aa8dbf1925ab92386edd4009923"
        );
    }

    #[test]
    fn the_published_blake2s_vectors_match() {
        // From RFC 7693 appendix B and the BLAKE2 reference vectors.
        assert_eq!(
            hex(&digest256(b"")),
            "69217a3079908094e11121d042354a7c1f55b6482ca1a51e1b250dfd1ed0eef9"
        );
        assert_eq!(
            hex(&digest256(b"abc")),
            "508c5e8c327c14e2e1a72ba34eeb452f37458b209ed63a294d999b4c86675982"
        );
    }

    #[test]
    fn an_exact_block_multiple_matches() {
        // 128 bytes is one BLAKE2b block and 64 is one BLAKE2s block, so each
        // message needs the trailing empty block the reference also compresses;
        // a `chunks`-only split would drop the final flag from the data block.
        assert_eq!(
            hex(&digest512(&[b'a'; 128])),
            "fc6c71f688f43ea7d60817478808f3cac753e61571865c95adbc2d9122c943a7\
             6b92c2cb1047ef3fe7bf6e436ec1d0a99a9e5b216780bf7fed9d7ca91d3a8f3b"
        );
        assert_eq!(
            hex(&digest256(&[b'a'; 64])),
            "651d2f5f20952eacaea2fba2f2af2bcd633e511ea2d2e4c9ae2ac0d9ffb7b252"
        );
    }
}
