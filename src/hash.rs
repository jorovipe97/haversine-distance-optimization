/// Offset basis for 64 bits
const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;

/// Prime for 64 bits FNV
const FNV_PRIME: u64 = 0x00000100000001b3;

/// Fowler–Noll–Vo hash function
///
/// See: https://en.wikipedia.org/wiki/Fowler%E2%80%93Noll%E2%80%93Vo_hash_function
pub fn fnv1a_hash(bytes: &[u8]) -> u64 {
    let mut hash: u64 = FNV_OFFSET_BASIS;

    for item in bytes {
        hash = hash ^ ((*item) as u64);
        hash = hash.wrapping_mul(FNV_PRIME);
    }

    hash
}
