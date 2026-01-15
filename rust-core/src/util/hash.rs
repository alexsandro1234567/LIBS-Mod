//! # Hash Utilities
//! 
//! Fast hashing for asset deduplication.

/// FNV-1a hash (fast, non-cryptographic)
pub fn fnv1a(data: &[u8]) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;
    
    let mut hash = FNV_OFFSET;
    for byte in data {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// XXHash-like fast hash
pub fn fast_hash(data: &[u8]) -> u64 {
    const PRIME1: u64 = 0x9E3779B185EBCA87;
    const PRIME2: u64 = 0xC2B2AE3D27D4EB4F;
    const PRIME3: u64 = 0x165667B19E3779F9;
    
    let mut h: u64 = data.len() as u64;
    
    for chunk in data.chunks(8) {
        let mut k = 0u64;
        for (i, &byte) in chunk.iter().enumerate() {
            k |= (byte as u64) << (i * 8);
        }
        h = h.wrapping_add(k.wrapping_mul(PRIME1));
        h = h.rotate_left(31);
        h = h.wrapping_mul(PRIME2);
    }
    
    h ^= h >> 33;
    h = h.wrapping_mul(PRIME3);
    h ^= h >> 29;
    
    h
}

/// Combine two hashes
pub fn hash_combine(h1: u64, h2: u64) -> u64 {
    h1 ^ (h2.wrapping_add(0x9e3779b9).wrapping_add(h1 << 6).wrapping_add(h1 >> 2))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_fnv1a() {
        let hash1 = fnv1a(b"hello");
        let hash2 = fnv1a(b"hello");
        let hash3 = fnv1a(b"world");
        
        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }
    
    #[test]
    fn test_fast_hash() {
        let hash1 = fast_hash(b"test data");
        let hash2 = fast_hash(b"test data");
        
        assert_eq!(hash1, hash2);
    }
}
