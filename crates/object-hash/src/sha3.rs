//! SHA-3, the FIPS 202 hash, for callers whose other system already speaks it.
//!
//! The reference package reaches SHA-3 through Node's `crypto`, so a service
//! that names `sha3-256` there and this crate here compute the same key. The
//! four fixed-length members share one sponge, differing only in the rate and
//! the output length, so they are one function with four entry points rather
//! than four copies of the round function.
//!
//! Like the other digests in this crate it names a value rather than protecting
//! it; it is not a signature and there is no key here.

/// The SHA3-224 digest of `data`.
pub(crate) fn digest224(data: &[u8]) -> [u8; 28] {
    let mut out = [0u8; 28];
    sponge(data, 144, &mut out);
    out
}

/// The SHA3-256 digest of `data`.
pub(crate) fn digest256(data: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    sponge(data, 136, &mut out);
    out
}

/// The SHA3-384 digest of `data`.
pub(crate) fn digest384(data: &[u8]) -> [u8; 48] {
    let mut out = [0u8; 48];
    sponge(data, 104, &mut out);
    out
}

/// The SHA3-512 digest of `data`.
pub(crate) fn digest512(data: &[u8]) -> [u8; 64] {
    let mut out = [0u8; 64];
    sponge(data, 72, &mut out);
    out
}

/// Round constants for the Keccak-f[1600] permutation, from FIPS 202.
const ROUND_CONSTANTS: [u64; 24] = [
    0x0000_0000_0000_0001,
    0x0000_0000_0000_8082,
    0x8000_0000_0000_808a,
    0x8000_0000_8000_8000,
    0x0000_0000_0000_808b,
    0x0000_0000_8000_0001,
    0x8000_0000_8000_8081,
    0x8000_0000_0000_8009,
    0x0000_0000_0000_008a,
    0x0000_0000_0000_0088,
    0x0000_0000_8000_8009,
    0x0000_0000_8000_000a,
    0x0000_0000_8000_808b,
    0x8000_0000_0000_008b,
    0x8000_0000_0000_8089,
    0x8000_0000_0000_8003,
    0x8000_0000_0000_8002,
    0x8000_0000_0000_0080,
    0x0000_0000_0000_800a,
    0x8000_0000_8000_000a,
    0x8000_0000_8000_8081,
    0x8000_0000_0000_8080,
    0x0000_0000_8000_0001,
    0x8000_0000_8000_8008,
];

/// The rho rotation offset for the lane at `x + 5 * y`.
const ROTATION: [u32; 25] = [
    0, 1, 62, 28, 27, //
    36, 44, 6, 55, 20, //
    3, 10, 43, 25, 39, //
    41, 45, 15, 21, 8, //
    18, 2, 61, 56, 14,
];

/// Absorbs `data` with the SHA-3 padding and squeezes `out.len()` bytes.
///
/// Every SHA-3 output length in this crate is at most its rate, so one squeeze
/// block is always enough and the state never has to be permuted again to fill
/// `out`.
fn sponge(data: &[u8], rate: usize, out: &mut [u8]) {
    let mut state = [0u64; 25];
    let mut blocks = data.chunks_exact(rate);
    for block in &mut blocks {
        absorb(&mut state, block);
        keccak_f(&mut state);
    }
    // The trailing bytes plus the domain-separation and pad bits. SHA-3, not
    // the original Keccak, so the first pad byte is 0x06 rather than 0x01.
    let mut last = [0u8; 200];
    let rest = blocks.remainder();
    last[..rest.len()].copy_from_slice(rest);
    last[rest.len()] = 0x06;
    last[rate - 1] |= 0x80;
    absorb(&mut state, &last[..rate]);
    keccak_f(&mut state);

    for (i, byte) in out.iter_mut().enumerate() {
        *byte = (state[i / 8] >> (8 * (i % 8))) as u8;
    }
}

/// XORs one rate-sized block into the state, little-endian within each lane.
fn absorb(state: &mut [u64; 25], block: &[u8]) {
    let (lanes, _) = block.as_chunks::<8>();
    for (lane, chunk) in state.iter_mut().zip(lanes) {
        *lane ^= u64::from_le_bytes(*chunk);
    }
}

/// The Keccak-f[1600] permutation: 24 rounds of theta, rho, pi and chi.
fn keccak_f(state: &mut [u64; 25]) {
    for round in ROUND_CONSTANTS {
        // theta
        let mut column = [0u64; 5];
        for x in 0..5 {
            column[x] = state[x] ^ state[x + 5] ^ state[x + 10] ^ state[x + 15] ^ state[x + 20];
        }
        for x in 0..5 {
            let d = column[(x + 4) % 5] ^ column[(x + 1) % 5].rotate_left(1);
            for y in 0..5 {
                state[x + 5 * y] ^= d;
            }
        }

        // rho and pi, combined: each lane moves to its permuted position while
        // being rotated.
        let mut moved = [0u64; 25];
        for x in 0..5 {
            for y in 0..5 {
                moved[y + 5 * ((2 * x + 3 * y) % 5)] =
                    state[x + 5 * y].rotate_left(ROTATION[x + 5 * y]);
            }
        }

        // chi
        for y in 0..5 {
            for x in 0..5 {
                state[x + 5 * y] =
                    moved[x + 5 * y] ^ (!moved[(x + 1) % 5 + 5 * y] & moved[(x + 2) % 5 + 5 * y]);
            }
        }

        // iota
        state[0] ^= round;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_published_abc_vectors_agree() {
        assert_eq!(
            hex(&digest224(b"abc")),
            "e642824c3f8cf24ad09234ee7d3c766fc9a3a5168d0c94ad73b46fdf"
        );
        assert_eq!(
            hex(&digest256(b"abc")),
            "3a985da74fe225b2045c172d6bd390bd855f086e3e9d525b46bfe24511431532"
        );
        assert_eq!(
            hex(&digest384(b"abc")),
            "ec01498288516fc926459f58e2c6ad8df9b473cb0fc08c2596da7cf0e49be4b298d88cea927ac7f539f1edf228376d25"
        );
        assert_eq!(
            hex(&digest512(b"abc")),
            "b751850b1a57168a5693cd924b6b096e08f621827444f70d884f5d0240d2712e10e116e9192af3c91a7ec57647e3934057340b4cf408d5a56592f8274eec53f0"
        );
    }

    #[test]
    fn the_empty_input_vectors_agree() {
        assert_eq!(
            hex(&digest256(b"")),
            "a7ffc6f8bf1ed76651c14756a061d662f580ff4de43b49fa82d80a4b80f8434a"
        );
    }

    /// A message longer than one block exercises the absorb loop and the
    /// padding at a block boundary.
    #[test]
    fn a_message_spanning_blocks_agrees() {
        let data = vec![0x61u8; 200];
        assert_eq!(
            hex(&digest256(&data)),
            "cce34485baf2bf2aca99b94833892a4f52896d3d153f7b840cc4f9fe695f1387"
        );
    }

    /// Lengths on either side of the SHA3-256 and SHA3-224 rate, where a
    /// message fills a block exactly and the padding starts a fresh one. An
    /// off-by-one in the pad byte would only show up here.
    #[test]
    fn lengths_around_the_rate_agree() {
        let expected = [
            (
                135,
                "8094bb53c44cfb1e67b7c30447f9a1c33696d2463ecc1d9c92538913392843c9",
                "f9f28c21a2b0884bbd3594cae82bf811c0c1ede427e083d5576e909d",
            ),
            (
                136,
                "3fc5559f14db8e453a0a3091edbd2bc25e11528d81c66fa570a4efdcc2695ee1",
                "96136a6a094433b4aa855f163829a2ce6bca7d56cfd2163b47f1f1c4",
            ),
            (
                137,
                "f8d6846cedd2ccfadf15c5879ef95af724d799eed7391fb1c91f95344e738614",
                "d0c9e8452b199b5149b9d06ec79e70ccd82ffa317bf61196f12b7207",
            ),
            (
                143,
                "2fca8e6f1fe530553ba362bcbdcb514588001280450c0d1a86bac02714dc9dcb",
                "73b1b22b54f515f626a6abdde6af25cd4801dc6e9dc7fa3f77e1c122",
            ),
            (
                144,
                "fe4206d88b37fb533f957c384ad50f2a7cb86a74eb53e4e591f1c07d5c719dcf",
                "f9019111996dcf160e284e320fd6d8825cabcd41a5ffdc4c5e9d64b6",
            ),
            (
                145,
                "ae4947f8dda0814cd69b4cdc87e350ee097bd41c761c1a367c3e1ed96e88090f",
                "7f0521c84aeacc8a46aba17171acbdd22522509a71c663257fbdee0e",
            ),
        ];
        for (length, sha256, sha224) in expected {
            let data = vec![0x61u8; length];
            assert_eq!(hex(&digest256(&data)), sha256, "sha3-256 of {length} bytes");
            assert_eq!(hex(&digest224(&data)), sha224, "sha3-224 of {length} bytes");
        }
    }

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }
}
