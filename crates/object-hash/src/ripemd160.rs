//! RIPEMD-160, for callers whose other system already speaks it.
//!
//! Written out for the same reason as the SHA family: the crate has no
//! dependencies, and the function is short. The reference package offers
//! `ripemd160` through Node's `crypto`, and this reproduces its digest for the
//! same canonical bytes. Like the others it names a value rather than protects
//! it: RIPEMD-160 is not a collision-resistant choice today, and the crate says
//! so rather than refusing the name.
//!
//! RIPEMD-160 runs two independent five-word lines over a 64-byte block and
//! combines them, so it shares no code with the SHA family even though the
//! block size is the same.

/// The RIPEMD-160 digest of `data`.
pub(crate) fn digest(data: &[u8]) -> [u8; 20] {
    let mut state = INIT;
    let bit_length = (data.len() as u64).wrapping_mul(8);

    let mut message = data.to_vec();
    message.push(0x80);
    // Little-endian length, so the padding fills to 56 mod 64.
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_length.to_le_bytes());

    for block in message.as_chunks::<64>().0 {
        let mut x = [0u32; 16];
        for (word, chunk) in x.iter_mut().zip(block.as_chunks::<4>().0) {
            *word = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        }

        let mut left = state;
        let mut right = state;
        for round in 0..80 {
            let group = round / 16;
            let (left_index, left_rotate) = LEFT[round];
            let temp = left[0]
                .wrapping_add(f(group, left[1], left[2], left[3]))
                .wrapping_add(x[left_index])
                .wrapping_add(LEFT_K[group]);
            let temp = temp.rotate_left(left_rotate).wrapping_add(left[4]);
            left = [left[4], temp, left[1], left[2].rotate_left(10), left[3]];

            // The right line runs the same five functions in the reverse order.
            let (right_index, right_rotate) = RIGHT[round];
            let temp = right[0]
                .wrapping_add(f(4 - group, right[1], right[2], right[3]))
                .wrapping_add(x[right_index])
                .wrapping_add(RIGHT_K[group]);
            let temp = temp.rotate_left(right_rotate).wrapping_add(right[4]);
            right = [right[4], temp, right[1], right[2].rotate_left(10), right[3]];
        }

        let combined = [
            state[1].wrapping_add(left[2]).wrapping_add(right[3]),
            state[2].wrapping_add(left[3]).wrapping_add(right[4]),
            state[3].wrapping_add(left[4]).wrapping_add(right[0]),
            state[4].wrapping_add(left[0]).wrapping_add(right[1]),
            state[0].wrapping_add(left[1]).wrapping_add(right[2]),
        ];
        state = combined;
    }

    let mut out = [0u8; 20];
    for (chunk, word) in out.as_chunks_mut::<4>().0.iter_mut().zip(state) {
        chunk.copy_from_slice(&word.to_le_bytes());
    }
    out
}

/// The initial state, the same five constants RIPEMD-128 starts from.
const INIT: [u32; 5] = [
    0x6745_2301,
    0xefcd_ab89,
    0x98ba_dcfe,
    0x1032_5476,
    0xc3d2_e1f0,
];

/// The added constant for each of the five rounds, left then right line.
const LEFT_K: [u32; 5] = [
    0x0000_0000,
    0x5a82_7999,
    0x6ed9_eba1,
    0x8f1b_bcdc,
    0xa953_fd4e,
];
const RIGHT_K: [u32; 5] = [
    0x50a2_8be6,
    0x5c4d_d124,
    0x6d70_3ef3,
    0x7a6d_76e9,
    0x0000_0000,
];

/// The message word and rotation for each of the 80 steps of the left line.
const LEFT: [(usize, u32); 80] = [
    (0, 11),
    (1, 14),
    (2, 15),
    (3, 12),
    (4, 5),
    (5, 8),
    (6, 7),
    (7, 9),
    (8, 11),
    (9, 13),
    (10, 14),
    (11, 15),
    (12, 6),
    (13, 7),
    (14, 9),
    (15, 8),
    (7, 7),
    (4, 6),
    (13, 8),
    (1, 13),
    (10, 11),
    (6, 9),
    (15, 7),
    (3, 15),
    (12, 7),
    (0, 12),
    (9, 15),
    (5, 9),
    (2, 11),
    (14, 7),
    (11, 13),
    (8, 12),
    (3, 11),
    (10, 13),
    (14, 6),
    (4, 7),
    (9, 14),
    (15, 9),
    (8, 13),
    (1, 15),
    (2, 14),
    (7, 8),
    (0, 13),
    (6, 6),
    (13, 5),
    (11, 12),
    (5, 7),
    (12, 5),
    (1, 11),
    (9, 12),
    (11, 14),
    (10, 15),
    (0, 14),
    (8, 15),
    (12, 9),
    (4, 8),
    (13, 9),
    (3, 14),
    (7, 5),
    (15, 6),
    (14, 8),
    (5, 6),
    (6, 5),
    (2, 12),
    (4, 9),
    (0, 15),
    (5, 5),
    (9, 11),
    (7, 6),
    (12, 8),
    (2, 13),
    (10, 12),
    (14, 5),
    (1, 12),
    (3, 13),
    (8, 14),
    (11, 11),
    (6, 8),
    (15, 5),
    (13, 6),
];

/// The message word and rotation for each of the 80 steps of the right line.
const RIGHT: [(usize, u32); 80] = [
    (5, 8),
    (14, 9),
    (7, 9),
    (0, 11),
    (9, 13),
    (2, 15),
    (11, 15),
    (4, 5),
    (13, 7),
    (6, 7),
    (15, 8),
    (8, 11),
    (1, 14),
    (10, 14),
    (3, 12),
    (12, 6),
    (6, 9),
    (11, 13),
    (3, 15),
    (7, 7),
    (0, 12),
    (13, 8),
    (5, 9),
    (10, 11),
    (14, 7),
    (15, 7),
    (8, 12),
    (12, 7),
    (4, 6),
    (9, 15),
    (1, 13),
    (2, 11),
    (15, 9),
    (5, 7),
    (1, 15),
    (3, 11),
    (7, 8),
    (14, 6),
    (6, 6),
    (9, 14),
    (11, 12),
    (8, 13),
    (12, 5),
    (2, 14),
    (10, 13),
    (0, 13),
    (4, 7),
    (13, 5),
    (8, 15),
    (6, 5),
    (4, 8),
    (1, 11),
    (3, 14),
    (11, 14),
    (15, 6),
    (0, 14),
    (5, 6),
    (12, 9),
    (2, 12),
    (13, 9),
    (9, 12),
    (7, 5),
    (10, 15),
    (14, 8),
    (12, 8),
    (15, 5),
    (10, 12),
    (4, 9),
    (1, 12),
    (5, 5),
    (8, 14),
    (7, 6),
    (6, 8),
    (2, 13),
    (13, 6),
    (14, 5),
    (0, 15),
    (3, 13),
    (9, 11),
    (11, 11),
];

/// The round-dependent boolean function, selected by round group (0 to 4).
fn f(group: usize, x: u32, y: u32, z: u32) -> u32 {
    match group {
        0 => x ^ y ^ z,
        1 => (x & y) | (!x & z),
        2 => (x | !y) ^ z,
        3 => (x & z) | (y & !z),
        _ => x ^ (y | !z),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The digest as lowercase hexadecimal, for the published vectors.
    fn hex(data: &[u8]) -> String {
        let mut out = String::with_capacity(40);
        for byte in digest(data) {
            out.push(char::from_digit(u32::from(byte >> 4), 16).expect("nibble"));
            out.push(char::from_digit(u32::from(byte & 0x0f), 16).expect("nibble"));
        }
        out
    }

    #[test]
    fn the_published_test_vectors_match() {
        // The vectors from the RIPEMD-160 paper and the common extras.
        assert_eq!(hex(b""), "9c1185a5c5e9fc54612808977ee8f548b2258d31");
        assert_eq!(hex(b"a"), "0bdc9d2d256b3ee9daae347be6f4dc835a467ffe");
        assert_eq!(hex(b"abc"), "8eb208f7e05d987a9b044a8e98c6b087f15a0bfc");
        assert_eq!(
            hex(b"message digest"),
            "5d0689ef49d2fae572b881b123a85ffa21595f36"
        );
        assert_eq!(
            hex(b"abcdefghijklmnopqrstuvwxyz"),
            "f71c27109c692c1b56bbdceb5b9d2865b3708dbc"
        );
        assert_eq!(
            hex(b"The quick brown fox jumps over the lazy dog"),
            "37f332f68db77bd9d7edd4969571ad671cf9dd3b"
        );
    }

    #[test]
    fn a_longer_message_spanning_many_blocks_matches() {
        // One million 'a', the standard long vector.
        let data = vec![b'a'; 1_000_000];
        assert_eq!(hex(&data), "52783243c1697bdbe16d37f97f68f08325dc1528");
    }
}
