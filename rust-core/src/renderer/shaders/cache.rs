//! # Shader Cache
//! 
//! Disk and memory caching for compiled shaders.

use std::collections::HashMap;
use std::path::PathBuf;
use std::io::{Read, Write};

/// Shader cache for storing compiled SPIR-V
pub struct ShaderCache {
    /// Cache directory
    cache_dir: Option<PathBuf>,
    /// In-memory cache
    memory_cache: HashMap<String, CacheEntry>,
    /// Maximum memory cache size (bytes)
    max_memory_size: usize,
    /// Current memory cache size
    current_memory_size: usize,
}

/// Cache entry
struct CacheEntry {
    /// Source hash
    source_hash: u64,
    /// Compiled SPIR-V
    spirv: Vec<u32>,
    /// Last access time
    last_access: std::time::Instant,
}

impl ShaderCache {
    /// Create a new shader cache
    pub fn new(cache_dir: Option<PathBuf>) -> Self {
        if let Some(ref dir) = cache_dir {
            let _ = std::fs::create_dir_all(dir);
        }
        
        Self {
            cache_dir,
            memory_cache: HashMap::new(),
            max_memory_size: 64 * 1024 * 1024, // 64 MB
            current_memory_size: 0,
        }
    }
    
    /// Get cached SPIR-V for a shader
    pub fn get(&self, key: &str, source: &str) -> Option<Vec<u32>> {
        let source_hash = Self::hash_source(source);
        
        // Check memory cache
        if let Some(entry) = self.memory_cache.get(key) {
            if entry.source_hash == source_hash {
                return Some(entry.spirv.clone());
            }
        }
        
        // Check disk cache
        if let Some(ref cache_dir) = self.cache_dir {
            let cache_path = cache_dir.join(format!("{}.spv", Self::sanitize_key(key)));
            if let Ok(spirv) = Self::read_cache_file(&cache_path, source_hash) {
                return Some(spirv);
            }
        }
        
        None
    }
    
    /// Store compiled SPIR-V in cache
    pub fn put(&mut self, key: &str, source: &str, spirv: &[u32]) {
        let source_hash = Self::hash_source(source);
        let spirv_size = spirv.len() * 4;
        
        // Evict if necessary
        while self.current_memory_size + spirv_size > self.max_memory_size && !self.memory_cache.is_empty() {
            self.evict_oldest();
        }
        
        // Add to memory cache
        let entry = CacheEntry {
            source_hash,
            spirv: spirv.to_vec(),
            last_access: std::time::Instant::now(),
        };
        
        if let Some(old) = self.memory_cache.insert(key.to_string(), entry) {
            self.current_memory_size -= old.spirv.len() * 4;
        }
        self.current_memory_size += spirv_size;
        
        // Write to disk cache
        if let Some(ref cache_dir) = self.cache_dir {
            let cache_path = cache_dir.join(format!("{}.spv", Self::sanitize_key(key)));
            let _ = Self::write_cache_file(&cache_path, source_hash, spirv);
        }
    }
    
    /// Clear all cached shaders
    pub fn clear(&mut self) {
        self.memory_cache.clear();
        self.current_memory_size = 0;
        
        if let Some(ref cache_dir) = self.cache_dir {
            if let Ok(entries) = std::fs::read_dir(cache_dir) {
                for entry in entries.flatten() {
                    if entry.path().extension().map_or(false, |e| e == "spv") {
                        let _ = std::fs::remove_file(entry.path());
                    }
                }
            }
        }
    }
    
    /// Invalidate a specific shader
    pub fn invalidate(&mut self, key: &str) {
        if let Some(entry) = self.memory_cache.remove(key) {
            self.current_memory_size -= entry.spirv.len() * 4;
        }
        
        if let Some(ref cache_dir) = self.cache_dir {
            let cache_path = cache_dir.join(format!("{}.spv", Self::sanitize_key(key)));
            let _ = std::fs::remove_file(cache_path);
        }
    }
    
    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            memory_entries: self.memory_cache.len(),
            memory_size: self.current_memory_size,
            max_memory_size: self.max_memory_size,
            disk_entries: self.count_disk_entries(),
        }
    }
    
    /// Evict oldest entry from memory cache
    fn evict_oldest(&mut self) {
        let oldest_key = self.memory_cache.iter()
            .min_by_key(|(_, entry)| entry.last_access)
            .map(|(key, _)| key.clone());
        
        if let Some(key) = oldest_key {
            if let Some(entry) = self.memory_cache.remove(&key) {
                self.current_memory_size -= entry.spirv.len() * 4;
            }
        }
    }
    
    /// Count disk cache entries
    fn count_disk_entries(&self) -> usize {
        if let Some(ref cache_dir) = self.cache_dir {
            if let Ok(entries) = std::fs::read_dir(cache_dir) {
                return entries
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().extension().map_or(false, |ext| ext == "spv"))
                    .count();
            }
        }
        0
    }
    
    /// Hash shader source for cache validation
    fn hash_source(source: &str) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        source.hash(&mut hasher);
        hasher.finish()
    }
    
    /// Sanitize cache key for filesystem
    fn sanitize_key(key: &str) -> String {
        key.chars()
            .map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
            .collect()
    }
    
    /// Read cache file
    fn read_cache_file(path: &PathBuf, expected_hash: u64) -> Result<Vec<u32>, std::io::Error> {
        let mut file = std::fs::File::open(path)?;
        
        // Read header
        let mut header = [0u8; 16];
        file.read_exact(&mut header)?;
        
        // Check magic number
        if &header[0..4] != b"SPVC" {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid cache file"));
        }
        
        // Check version
        let version = u32::from_le_bytes([header[4], header[5], header[6], header[7]]);
        if version != 1 {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Unsupported cache version"));
        }
        
        // Check source hash
        let stored_hash = u64::from_le_bytes([
            header[8], header[9], header[10], header[11],
            header[12], header[13], header[14], header[15],
        ]);
        
        if stored_hash != expected_hash {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Source hash mismatch"));
        }
        
        // Read SPIR-V data
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        
        // Convert to u32
        let spirv: Vec<u32> = data.chunks_exact(4)
            .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect();
        
        Ok(spirv)
    }
    
    /// Write cache file
    fn write_cache_file(path: &PathBuf, source_hash: u64, spirv: &[u32]) -> Result<(), std::io::Error> {
        let mut file = std::fs::File::create(path)?;
        
        // Write header
        file.write_all(b"SPVC")?; // Magic number
        file.write_all(&1u32.to_le_bytes())?; // Version
        file.write_all(&source_hash.to_le_bytes())?; // Source hash
        
        // Write SPIR-V data
        for word in spirv {
            file.write_all(&word.to_le_bytes())?;
        }
        
        Ok(())
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Number of entries in memory cache
    pub memory_entries: usize,
    /// Current memory cache size in bytes
    pub memory_size: usize,
    /// Maximum memory cache size in bytes
    pub max_memory_size: usize,
    /// Number of entries in disk cache
    pub disk_entries: usize,
}
