//! Void Manager - Off-Heap Memory Management
//!
//! Moves assets outside Java's GC-managed heap:
//! - Textures, sounds, models in native RAM
//! - Java keeps only 8-byte pointers
//! - Eliminates GC pauses

use std::sync::Arc;
use std::collections::HashMap;
use parking_lot::RwLock;
use std::ptr::NonNull;
use std::alloc::{alloc, dealloc, Layout};

/// Memory handle (8 bytes, stored in Java)
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VoidHandle {
    pub ptr: u64,
    pub size: u32,
    pub type_id: u16,
    pub flags: u16,
}

/// Asset types
#[repr(u16)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssetType {
    Unknown = 0,
    Texture = 1,
    Sound = 2,
    Model = 3,
    Shader = 4,
    BlockData = 5,
    EntityData = 6,
    ChunkMesh = 7,
}

/// Void Manager - Off-heap memory allocator
pub struct VoidManager {
    /// All allocations
    allocations: RwLock<HashMap<u64, AllocationInfo>>,
    /// Asset deduplication map (hash -> handle)
    dedup_map: RwLock<HashMap<u64, VoidHandle>>,
    /// Statistics
    stats: RwLock<VoidStats>,
    /// Next allocation ID
    next_id: std::sync::atomic::AtomicU64,
}

/// Allocation metadata
struct AllocationInfo {
    ptr: NonNull<u8>,
    layout: Layout,
    asset_type: AssetType,
    ref_count: u32,
    hash: u64,
}

/// Memory statistics
#[derive(Default, Clone)]
pub struct VoidStats {
    pub total_allocated: u64,
    pub total_freed: u64,
    pub current_usage: u64,
    pub allocation_count: u32,
    pub dedup_hits: u32,
    pub dedup_savings: u64,
    pub peak_usage: u64,
}

impl VoidManager {
    /// Create new Void Manager
    pub fn new() -> Arc<Self> {
        log::info!("Initializing Void Manager (Off-Heap Memory)");
        
        Arc::new(Self {
            allocations: RwLock::new(HashMap::new()),
            dedup_map: RwLock::new(HashMap::new()),
            stats: RwLock::new(VoidStats::default()),
            next_id: std::sync::atomic::AtomicU64::new(1),
        })
    }
    
    /// Allocate memory for asset
    pub fn allocate(&self, size: usize, asset_type: AssetType) -> Result<VoidHandle, VoidError> {
        if size == 0 {
            return Err(VoidError::InvalidSize);
        }
        
        let layout = Layout::from_size_align(size, 16)
            .map_err(|_| VoidError::InvalidAlignment)?;
        
        let ptr = unsafe { alloc(layout) };
        
        if ptr.is_null() {
            return Err(VoidError::OutOfMemory);
        }
        
        let ptr_nonnull = NonNull::new(ptr).unwrap();
        let id = self.next_id.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        
        let handle = VoidHandle {
            ptr: ptr as u64,
            size: size as u32,
            type_id: asset_type as u16,
            flags: 0,
        };
        
        // Store allocation
        {
            let mut allocations = self.allocations.write();
            allocations.insert(handle.ptr, AllocationInfo {
                ptr: ptr_nonnull,
                layout,
                asset_type,
                ref_count: 1,
                hash: 0,
            });
        }
        
        // Update stats
        {
            let mut stats = self.stats.write();
            stats.total_allocated += size as u64;
            stats.current_usage += size as u64;
            stats.allocation_count += 1;
            stats.peak_usage = stats.peak_usage.max(stats.current_usage);
        }
        
        Ok(handle)
    }
    
    /// Allocate with deduplication
    pub fn allocate_dedup(&self, data: &[u8], asset_type: AssetType) -> Result<VoidHandle, VoidError> {
        // Calculate hash
        let hash = Self::hash_data(data);
        
        // Check dedup map
        {
            let dedup = self.dedup_map.read();
            if let Some(&handle) = dedup.get(&hash) {
                // Increment ref count
                let mut allocations = self.allocations.write();
                if let Some(info) = allocations.get_mut(&handle.ptr) {
                    info.ref_count += 1;
                }
                
                let mut stats = self.stats.write();
                stats.dedup_hits += 1;
                stats.dedup_savings += data.len() as u64;
                
                return Ok(handle);
            }
        }
        
        // Allocate new
        let handle = self.allocate(data.len(), asset_type)?;
        
        // Copy data
        unsafe {
            std::ptr::copy_nonoverlapping(data.as_ptr(), handle.ptr as *mut u8, data.len());
        }
        
        // Store hash
        {
            let mut allocations = self.allocations.write();
            if let Some(info) = allocations.get_mut(&handle.ptr) {
                info.hash = hash;
            }
        }
        
        // Add to dedup map
        {
            let mut dedup = self.dedup_map.write();
            dedup.insert(hash, handle);
        }
        
        Ok(handle)
    }
    
    /// Free memory
    pub fn free(&self, handle: VoidHandle) -> Result<(), VoidError> {
        let mut allocations = self.allocations.write();
        
        if let Some(info) = allocations.get_mut(&handle.ptr) {
            info.ref_count = info.ref_count.saturating_sub(1);
            
            if info.ref_count == 0 {
                // Actually free
                unsafe {
                    dealloc(info.ptr.as_ptr(), info.layout);
                }
                
                // Remove from dedup map if present
                if info.hash != 0 {
                    let mut dedup = self.dedup_map.write();
                    dedup.remove(&info.hash);
                }
                
                let size = info.layout.size() as u64;
                allocations.remove(&handle.ptr);
                
                // Update stats
                let mut stats = self.stats.write();
                stats.total_freed += size;
                stats.current_usage -= size;
            }
            
            Ok(())
        } else {
            Err(VoidError::InvalidHandle)
        }
    }
    
    /// Get data pointer
    pub fn get_ptr(&self, handle: VoidHandle) -> Option<*const u8> {
        let allocations = self.allocations.read();
        allocations.get(&handle.ptr).map(|_| handle.ptr as *const u8)
    }
    
    /// Get mutable data pointer
    pub fn get_ptr_mut(&self, handle: VoidHandle) -> Option<*mut u8> {
        let allocations = self.allocations.read();
        allocations.get(&handle.ptr).map(|_| handle.ptr as *mut u8)
    }
    
    /// Read data from handle
    pub fn read(&self, handle: VoidHandle) -> Option<Vec<u8>> {
        let allocations = self.allocations.read();
        allocations.get(&handle.ptr).map(|info| {
            let mut data = vec![0u8; info.layout.size()];
            unsafe {
                std::ptr::copy_nonoverlapping(
                    handle.ptr as *const u8,
                    data.as_mut_ptr(),
                    data.len()
                );
            }
            data
        })
    }
    
    /// Write data to handle
    pub fn write(&self, handle: VoidHandle, data: &[u8]) -> Result<(), VoidError> {
        let allocations = self.allocations.read();
        
        if let Some(info) = allocations.get(&handle.ptr) {
            if data.len() > info.layout.size() {
                return Err(VoidError::BufferOverflow);
            }
            
            unsafe {
                std::ptr::copy_nonoverlapping(
                    data.as_ptr(),
                    handle.ptr as *mut u8,
                    data.len()
                );
            }
            
            Ok(())
        } else {
            Err(VoidError::InvalidHandle)
        }
    }
    
    /// Hash data for deduplication
    fn hash_data(data: &[u8]) -> u64 {
        use std::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;
        
        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);
        hasher.finish()
    }
    
    /// Get statistics
    pub fn get_stats(&self) -> VoidStats {
        self.stats.read().clone()
    }
    
    /// Garbage collect (free unreferenced allocations)
    pub fn gc(&self) -> u32 {
        let mut freed = 0;
        let mut to_free = Vec::new();
        
        {
            let allocations = self.allocations.read();
            for (&ptr, info) in allocations.iter() {
                if info.ref_count == 0 {
                    to_free.push(VoidHandle {
                        ptr,
                        size: info.layout.size() as u32,
                        type_id: info.asset_type as u16,
                        flags: 0,
                    });
                }
            }
        }
        
        for handle in to_free {
            if self.free(handle).is_ok() {
                freed += 1;
            }
        }
        
        freed
    }
    
    /// Clear all allocations (shutdown)
    pub fn clear(&self) {
        let mut allocations = self.allocations.write();
        
        for (_, info) in allocations.drain() {
            unsafe {
                dealloc(info.ptr.as_ptr(), info.layout);
            }
        }
        
        self.dedup_map.write().clear();
        *self.stats.write() = VoidStats::default();
        
        log::info!("Void Manager cleared all allocations");
    }
}

impl Drop for VoidManager {
    fn drop(&mut self) {
        self.clear();
    }
}

/// Void Manager errors
#[derive(Debug)]
pub enum VoidError {
    OutOfMemory,
    InvalidSize,
    InvalidAlignment,
    InvalidHandle,
    BufferOverflow,
}

impl std::fmt::Display for VoidError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OutOfMemory => write!(f, "Out of native memory"),
            Self::InvalidSize => write!(f, "Invalid allocation size"),
            Self::InvalidAlignment => write!(f, "Invalid memory alignment"),
            Self::InvalidHandle => write!(f, "Invalid void handle"),
            Self::BufferOverflow => write!(f, "Buffer overflow"),
        }
    }
}

impl std::error::Error for VoidError {}

/// JNI-safe handle for Java interop
#[repr(C)]
pub struct JniVoidHandle {
    pub handle_low: u32,
    pub handle_high: u32,
    pub size: u32,
    pub type_id: u32,
}

impl From<VoidHandle> for JniVoidHandle {
    fn from(h: VoidHandle) -> Self {
        Self {
            handle_low: h.ptr as u32,
            handle_high: (h.ptr >> 32) as u32,
            size: h.size,
            type_id: h.type_id as u32,
        }
    }
}

impl From<JniVoidHandle> for VoidHandle {
    fn from(j: JniVoidHandle) -> Self {
        Self {
            ptr: (j.handle_low as u64) | ((j.handle_high as u64) << 32),
            size: j.size,
            type_id: j.type_id as u16,
            flags: 0,
        }
    }
}
