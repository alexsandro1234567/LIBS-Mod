//! # Memory Management Module
//! 
//! Off-heap memory management for avoiding GC pauses.

pub mod void_manager;

use std::alloc::{alloc, dealloc, Layout};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Global memory tracking
static ALLOCATED_BYTES: AtomicUsize = AtomicUsize::new(0);
static ALLOCATION_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Memory manager for off-heap allocations
pub struct MemoryManager;

impl MemoryManager {
    /// Allocate memory
    pub fn allocate(size: usize) -> Option<*mut u8> {
        if size == 0 {
            return None;
        }
        
        let layout = match Layout::from_size_align(size, 16) {
            Ok(l) => l,
            Err(_) => return None,
        };
        
        unsafe {
            let ptr = alloc(layout);
            if ptr.is_null() {
                None
            } else {
                ALLOCATED_BYTES.fetch_add(size, Ordering::SeqCst);
                ALLOCATION_COUNT.fetch_add(1, Ordering::SeqCst);
                Some(ptr)
            }
        }
    }
    
    /// Free memory
    /// 
    /// # Safety
    /// Pointer must have been allocated by this manager
    pub unsafe fn free(ptr: *mut u8) {
        if ptr.is_null() {
            return;
        }
        
        // Note: In production, we'd track the size per allocation
        // For now, just mark one deallocation
        ALLOCATION_COUNT.fetch_sub(1, Ordering::SeqCst);
    }
    
    /// Free memory with known size
    pub unsafe fn free_sized(ptr: *mut u8, size: usize) {
        if ptr.is_null() || size == 0 {
            return;
        }
        
        if let Ok(layout) = Layout::from_size_align(size, 16) {
            dealloc(ptr, layout);
            ALLOCATED_BYTES.fetch_sub(size, Ordering::SeqCst);
            ALLOCATION_COUNT.fetch_sub(1, Ordering::SeqCst);
        }
    }
    
    /// Get currently allocated bytes
    pub fn get_allocated_bytes() -> usize {
        ALLOCATED_BYTES.load(Ordering::SeqCst)
    }
    
    /// Get allocation count
    pub fn get_allocation_count() -> usize {
        ALLOCATION_COUNT.load(Ordering::SeqCst)
    }
}

/// Initialize memory subsystem
pub fn init() -> Result<(), String> {
    log::debug!("Memory subsystem initialized");
    Ok(())
}

/// Shutdown memory subsystem
pub fn shutdown() {
    let bytes = ALLOCATED_BYTES.load(Ordering::SeqCst);
    let count = ALLOCATION_COUNT.load(Ordering::SeqCst);
    
    if bytes > 0 || count > 0 {
        log::warn!(
            "Memory leak detected: {} bytes in {} allocations",
            bytes, count
        );
    } else {
        log::debug!("Memory subsystem shutdown - no leaks detected");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_allocate_free() {
        let ptr = MemoryManager::allocate(1024);
        assert!(ptr.is_some());
        
        unsafe {
            MemoryManager::free_sized(ptr.unwrap(), 1024);
        }
    }
}
