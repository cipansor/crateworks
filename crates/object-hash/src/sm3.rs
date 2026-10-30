//! SM3, the Chinese national hash standard, for callers whose other system
//! already speaks it.
//!
//! Written out for the same reason as the SHA family: the crate has no
//! dependencies, and the function is short. The reference package offers `sm3`
//! through Node's `crypto`, and this reproduces its digest for the same
//! canonical bytes.
//!
//! SM3 borrows SHA-256's shape - 32-bit words, 64-byte blocks, big-endian
//! length - but has its own round function, message expansion and constants, so
//! it shares no code with [`crate::sha256`].

/// The SM3 digest of `data`.
pub(crate) fn digest(data: &[u8]) -> [u8; 32] {
    let mut state = INIT;
    let bit_length = (data.len() as u64).wrapping_mul(8);

    let mut message = data.to_vec();
    message.push(0x80);
    // Big-endian length, so the padding fills to 56 mod 64.
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_length.to_be_bytes());

    for block in message.as_chunks::<64>().0 {
        let mut w = [0u32; 68];
        for (word, chunk) in w.iter_mut().zip(block.as_chunks::<4>().0) {
            *word = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        }
        for index in 16..68 {
            let value = w[index - 16] ^ w[index - 9] ^ w[index - 3].rotate_left(15);
            w[index] = p1(value) ^ w[index - 13].rotate_left(7) ^ w[index - 6];
        }

        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = state;
        for index in 0..64 {
            let constant: u32 = if index < 16 { 0x79cc_4519 } else { 0x7a87_9d8a };
            let a12 = a.rotate_left(12);
            let ss1 = a12
                .wrapping_add(e)
                .wrapping_add(constant.rotate_left(index as u32))
                .rotate_left(7);
            let ss2 = ss1 ^ a12;
            let tt1 = ff(index, a, b, c)
                .wrapping_add(d)
                .wrapping_add(ss2)
                .wrapping_add(w[index] ^ w[index + 4]);
            let tt2 = gg(index, e, f, g)
                .wrapping_add(h)
                .wrapping_add(ss1)
                .wrapping_add(w[index]);

            d = c;
            c = b.rotate_left(9);
            b = a;
            a = tt1;
            h = g;
            g = f.rotate_left(19);
            f = e;
            e = p0(tt2);
        }

        for (slot, value) in state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *slot ^= value;
        }
    }

    let mut out = [0u8; 32];
    for (chunk, word) in out.as_chunks_mut::<4>().0.iter_mut().zip(state) {
        chunk.copy_from_slice(&word.to_be_bytes());
    }
    out
}

/// The initial state, from GB/T 32905-2016.
const INIT: [u32; 8] = [
    0x7380_166f,
    0x4914_b2b9,
    0x1724_42d7,
    0xda8a_0600,
    0xa96f_30bc,
    0x1631_38aa,
    0xe38d_ee4d,
    0xb0fb_0e4e,
];

/// The permutation `P0` applied after the second round sum.
fn p0(value: u32) -> u32 {
    value ^ value.rotate_left(9) ^ value.rotate_left(17)
}

/// The permutation `P1` applied during message expansion.
fn p1(value: u32) -> u32 {
    value ^ value.rotate_left(15) ^ value.rotate_left(23)
}

/// The round-dependent boolean function `FF`.
fn ff(round: usize, x: u32, y: u32, z: u32) -> u32 {
    if round < 16 {
        x ^ y ^ z
    } else {
        (x & y) | (x & z) | (y & z)
    }
}

/// The round-dependent boolean function `GG`.
fn gg(round: usize, x: u32, y: u32, z: u32) -> u32 {
    if round < 16 {
        x ^ y ^ z
    } else {
        (x & y) | (!x & z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The digest as lowercase hexadecimal, for the published vectors.
    fn hex(data: &[u8]) -> String {
        let mut out = String::with_capacity(64);
        for byte in digest(data) {
            out.push(char::from_digit(u32::from(byte >> 4), 16).expect("nibble"));
            out.push(char::from_digit(u32::from(byte & 0x0f), 16).expect("nibble"));
        }
        out
    }

    #[test]
    fn the_published_test_vectors_match() {
        // The vectors from GB/T 32905-2016, appendix A.
        assert_eq!(
            hex(b"abc"),
            "66c7f0f462eeedd9d1f2d46bdc10e4e24167c4875cf2f7a2297da02b8f4ba8e0"
        );
        assert_eq!(
            hex(b"abcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcd"),
            "debe9ff92275b8a138604889c18e5a4d6fdb70e5387e5765293dcba39c0c5732"
        );
    }
}
