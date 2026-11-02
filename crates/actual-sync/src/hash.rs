//! MurmurHash3 32-bit implementation compatible with upstream Actual Budget (`murmurhash.v3`).

/// Computes the 32-bit MurmurHash3 (x86_32) of a byte slice with the given seed.
#[must_use]
pub fn murmur3_32(key: &[u8], seed: u32) -> u32 {
    let c1: u32 = 0xcc9e_2d51;
    let c2: u32 = 0x1b87_3593;

    let remainder = key.len() & 3;
    let bytes = key.len() - remainder;

    let mut h1 = seed;
    let mut i = 0;

    while i < bytes {
        let mut k1 = (key[i] as u32)
            | ((key[i + 1] as u32) << 8)
            | ((key[i + 2] as u32) << 16)
            | ((key[i + 3] as u32) << 24);
        i += 4;

        k1 = k1.wrapping_mul(c1);
        k1 = k1.rotate_left(15);
        k1 = k1.wrapping_mul(c2);

        h1 ^= k1;
        h1 = h1.rotate_left(13);
        h1 = h1.wrapping_mul(5).wrapping_add(0xe654_6b64);
    }

    let mut k1 = 0u32;
    if remainder == 3 {
        k1 ^= (key[i + 2] as u32) << 16;
    }
    if remainder >= 2 {
        k1 ^= (key[i + 1] as u32) << 8;
    }
    if remainder >= 1 {
        k1 ^= key[i] as u32;

        k1 = k1.wrapping_mul(c1);
        k1 = k1.rotate_left(15);
        k1 = k1.wrapping_mul(c2);
        h1 ^= k1;
    }

    h1 ^= key.len() as u32;

    h1 ^= h1 >> 16;
    h1 = h1.wrapping_mul(0x85eb_ca6b);
    h1 ^= h1 >> 13;
    h1 = h1.wrapping_mul(0xc2b2_ae35);
    h1 ^= h1 >> 16;

    h1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_string_hash() {
        assert_eq!(murmur3_32(b"", 0), 0);
    }

    #[test]
    fn test_known_upstream_timestamp_hash() {
        let ts = "2018-11-12T13:21:40.122Z-0000-0123456789ABCDEF";
        assert_eq!(murmur3_32(ts.as_bytes(), 0), 1_983_295_247);
    }
}
