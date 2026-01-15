//! # Memory Module Unit Tests
//! 
//! Unit tests for memory management and allocation.

use aether_core::memory::*;

#[cfg(test)]
mod pool_allocator_tests {
    use super::*;
    
    #[test]
    fn test_pool_creation() {
        let pool = PoolAllocator::new(64, 100);
        assert_eq!(pool.block_size(), 64);
        assert_eq!(pool.capacity(), 100);
        assert_eq!(pool.available(), 100);
    }
    
    #[test]
    fn test_single_allocation() {
        let mut pool = PoolAllocator::new(64, 100);
        
        let ptr = pool.allocate();
        assert!(ptr.is_some());
        assert_eq!(pool.available(), 99);
    }
    
    #[test]
    fn test_allocation_and_deallocation() {
        let mut pool = PoolAllocator::new(64, 100);
        
        let ptr = pool.allocate().unwrap();
        assert_eq!(pool.available(), 99);
        
        pool.deallocate(ptr);
        assert_eq!(pool.available(), 100);
    }
    
    #[test]
    fn test_multiple_allocations() {
        let mut pool = PoolAllocator::new(64, 10);
        let mut ptrs = Vec::new();
        
        for _ in 0..10 {
            let ptr = pool.allocate();
            assert!(ptr.is_some());
            ptrs.push(ptr.unwrap());
        }
        
        assert_eq!(pool.available(), 0);
        
        // Should fail when pool is exhausted
        assert!(pool.allocate().is_none());
    }
    
    #[test]
    fn test_reuse_after_deallocation() {
        let mut pool = PoolAllocator::new(64, 1);
        
        let ptr1 = pool.allocate().unwrap();
        pool.deallocate(ptr1);
        
        let ptr2 = pool.allocate();
        assert!(ptr2.is_some());
    }
    
    #[test]
    fn test_pool_reset() {
        let mut pool = PoolAllocator::new(64, 10);
        
        for _ in 0..10 {
            pool.allocate();
        }
        
        assert_eq!(pool.available(), 0);
        
        pool.reset();
        
        assert_eq!(pool.available(), 10);
    }
}

#[cfg(test)]
mod arena_allocator_tests {
    use super::*;
    
    #[test]
    fn test_arena_creation() {
        let arena = ArenaAllocator::new(1024);
        assert_eq!(arena.capacity(), 1024);
        assert_eq!(arena.used(), 0);
    }
    
    #[test]
    fn test_arena_allocation() {
        let mut arena = ArenaAllocator::new(1024);
        
        let ptr = arena.allocate(64, 8);
        assert!(ptr.is_some());
        assert!(arena.used() >= 64);
    }
    
    #[test]
    fn test_arena_alignment() {
        let mut arena = ArenaAllocator::new(1024);
        
        // Allocate with 16-byte alignment
        let ptr = arena.allocate(32, 16).unwrap();
        assert_eq!(ptr as usize % 16, 0);
        
        // Allocate with 32-byte alignment
        let ptr = arena.allocate(32, 32).unwrap();
        assert_eq!(ptr as usize % 32, 0);
    }
    
    #[test]
    fn test_arena_sequential_allocations() {
        let mut arena = ArenaAllocator::new(1024);
        
        let ptr1 = arena.allocate(64, 8).unwrap();
        let ptr2 = arena.allocate(64, 8).unwrap();
        let ptr3 = arena.allocate(64, 8).unwrap();
        
        // Pointers should be sequential (with alignment)
        assert!(ptr2 as usize > ptr1 as usize);
        assert!(ptr3 as usize > ptr2 as usize);
    }
    
    #[test]
    fn test_arena_exhaustion() {
        let mut arena = ArenaAllocator::new(128);
        
        let _ = arena.allocate(64, 8);
        let _ = arena.allocate(64, 8);
        
        // Should fail when arena is exhausted
        let result = arena.allocate(64, 8);
        assert!(result.is_none());
    }
    
    #[test]
    fn test_arena_reset() {
        let mut arena = ArenaAllocator::new(1024);
        
        arena.allocate(512, 8);
        assert!(arena.used() >= 512);
        
        arena.reset();
        assert_eq!(arena.used(), 0);
    }
    
    #[test]
    fn test_arena_typed_allocation() {
        let mut arena = ArenaAllocator::new(1024);
        
        #[repr(C)]
        struct TestStruct {
            a: u64,
            b: u32,
            c: u16,
        }
        
        let ptr: *mut TestStruct = arena.allocate_typed();
        assert!(ptr.is_some());
        
        unsafe {
            (*ptr.unwrap()).a = 42;
            (*ptr.unwrap()).b = 100;
            (*ptr.unwrap()).c = 200;
            
            assert_eq!((*ptr.unwrap()).a, 42);
            assert_eq!((*ptr.unwrap()).b, 100);
            assert_eq!((*ptr.unwrap()).c, 200);
        }
    }
}

#[cfg(test)]
mod ring_buffer_tests {
    use super::*;
    
    #[test]
    fn test_ring_buffer_creation() {
        let buffer: RingBuffer<u32> = RingBuffer::new(10);
        assert_eq!(buffer.capacity(), 10);
        assert_eq!(buffer.len(), 0);
        assert!(buffer.is_empty());
    }
    
    #[test]
    fn test_ring_buffer_push_pop() {
        let mut buffer: RingBuffer<u32> = RingBuffer::new(10);
        
        buffer.push(1);
        buffer.push(2);
        buffer.push(3);
        
        assert_eq!(buffer.len(), 3);
        
        assert_eq!(buffer.pop(), Some(1));
        assert_eq!(buffer.pop(), Some(2));
        assert_eq!(buffer.pop(), Some(3));
        assert_eq!(buffer.pop(), None);
    }
    
    #[test]
    fn test_ring_buffer_wraparound() {
        let mut buffer: RingBuffer<u32> = RingBuffer::new(3);
        
        buffer.push(1);
        buffer.push(2);
        buffer.push(3);
        
        // Buffer is full, oldest should be overwritten
        buffer.push(4);
        
        assert_eq!(buffer.pop(), Some(2));
        assert_eq!(buffer.pop(), Some(3));
        assert_eq!(buffer.pop(), Some(4));
    }
    
    #[test]
    fn test_ring_buffer_peek() {
        let mut buffer: RingBuffer<u32> = RingBuffer::new(10);
        
        buffer.push(42);
        
        assert_eq!(buffer.peek(), Some(&42));
        assert_eq!(buffer.len(), 1); // Peek doesn't remove
        
        assert_eq!(buffer.pop(), Some(42));
        assert_eq!(buffer.peek(), None);
    }
    
    #[test]
    fn test_ring_buffer_clear() {
        let mut buffer: RingBuffer<u32> = RingBuffer::new(10);
        
        for i in 0..5 {
            buffer.push(i);
        }
        
        assert_eq!(buffer.len(), 5);
        
        buffer.clear();
        
        assert_eq!(buffer.len(), 0);
        assert!(buffer.is_empty());
    }
}

#[cfg(test)]
mod memory_pool_tests {
    use super::*;
    
    #[test]
    fn test_typed_pool() {
        #[derive(Debug, Clone, PartialEq)]
        struct Entity {
            id: u32,
            x: f32,
            y: f32,
        }
        
        let mut pool: TypedPool<Entity> = TypedPool::new(100);
        
        let handle = pool.allocate(Entity { id: 1, x: 0.0, y: 0.0 });
        assert!(handle.is_valid());
        
        let entity = pool.get(handle);
        assert!(entity.is_some());
        assert_eq!(entity.unwrap().id, 1);
    }
    
    #[test]
    fn test_typed_pool_mutation() {
        #[derive(Debug, Clone)]
        struct Counter {
            value: u32,
        }
        
        let mut pool: TypedPool<Counter> = TypedPool::new(10);
        
        let handle = pool.allocate(Counter { value: 0 });
        
        {
            let counter = pool.get_mut(handle).unwrap();
            counter.value += 1;
        }
        
        assert_eq!(pool.get(handle).unwrap().value, 1);
    }
    
    #[test]
    fn test_typed_pool_deallocation() {
        #[derive(Debug, Clone)]
        struct Data {
            value: u32,
        }
        
        let mut pool: TypedPool<Data> = TypedPool::new(10);
        
        let handle = pool.allocate(Data { value: 42 });
        assert!(pool.get(handle).is_some());
        
        pool.deallocate(handle);
        assert!(pool.get(handle).is_none());
    }
}

#[cfg(test)]
mod slab_allocator_tests {
    use super::*;
    
    #[test]
    fn test_slab_allocator() {
        let mut slab = SlabAllocator::new(&[16, 32, 64, 128, 256]);
        
        // Allocate different sizes
        let ptr16 = slab.allocate(16);
        let ptr32 = slab.allocate(32);
        let ptr64 = slab.allocate(64);
        
        assert!(ptr16.is_some());
        assert!(ptr32.is_some());
        assert!(ptr64.is_some());
    }
    
    #[test]
    fn test_slab_size_classes() {
        let mut slab = SlabAllocator::new(&[16, 32, 64]);
        
        // Request 20 bytes, should get 32-byte block
        let ptr = slab.allocate(20);
        assert!(ptr.is_some());
        
        // Request 50 bytes, should get 64-byte block
        let ptr = slab.allocate(50);
        assert!(ptr.is_some());
    }
    
    #[test]
    fn test_slab_deallocation() {
        let mut slab = SlabAllocator::new(&[32]);
        
        let ptr = slab.allocate(32).unwrap();
        let initial_available = slab.available(32);
        
        slab.deallocate(ptr, 32);
        
        assert_eq!(slab.available(32), initial_available + 1);
    }
}

#[cfg(test)]
mod buddy_allocator_tests {
    use super::*;
    
    #[test]
    fn test_buddy_allocator_creation() {
        let buddy = BuddyAllocator::new(1024 * 1024); // 1 MB
        assert!(buddy.total_size() >= 1024 * 1024);
    }
    
    #[test]
    fn test_buddy_allocation() {
        let mut buddy = BuddyAllocator::new(1024 * 1024);
        
        let ptr = buddy.allocate(4096);
        assert!(ptr.is_some());
    }
    
    #[test]
    fn test_buddy_power_of_two() {
        let mut buddy = BuddyAllocator::new(1024 * 1024);
        
        // Request non-power-of-two, should round up
        let ptr = buddy.allocate(1000);
        assert!(ptr.is_some());
        
        // Actual allocation should be 1024 (next power of 2)
    }
    
    #[test]
    fn test_buddy_coalescing() {
        let mut buddy = BuddyAllocator::new(1024);
        
        let ptr1 = buddy.allocate(256).unwrap();
        let ptr2 = buddy.allocate(256).unwrap();
        
        buddy.deallocate(ptr1, 256);
        buddy.deallocate(ptr2, 256);
        
        // After deallocating both buddies, should be able to allocate 512
        let ptr3 = buddy.allocate(512);
        assert!(ptr3.is_some());
    }
}

#[cfg(test)]
mod memory_stats_tests {
    use super::*;
    
    #[test]
    fn test_memory_stats_tracking() {
        let mut stats = MemoryStats::new();
        
        stats.record_allocation("test", 1024);
        stats.record_allocation("test", 2048);
        
        assert_eq!(stats.total_allocated("test"), 3072);
        assert_eq!(stats.allocation_count("test"), 2);
    }
    
    #[test]
    fn test_memory_stats_deallocation() {
        let mut stats = MemoryStats::new();
        
        stats.record_allocation("test", 1024);
        stats.record_deallocation("test", 512);
        
        assert_eq!(stats.current_usage("test"), 512);
    }
    
    #[test]
    fn test_memory_stats_peak() {
        let mut stats = MemoryStats::new();
        
        stats.record_allocation("test", 1024);
        stats.record_allocation("test", 1024);
        stats.record_deallocation("test", 1024);
        
        assert_eq!(stats.peak_usage("test"), 2048);
        assert_eq!(stats.current_usage("test"), 1024);
    }
}
