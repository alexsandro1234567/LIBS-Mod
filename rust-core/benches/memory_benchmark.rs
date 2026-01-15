//! # Memory Benchmarks
//! 
//! Performance benchmarks for memory allocation systems.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use std::alloc::{alloc, dealloc, Layout};

fn bench_pool_allocator(c: &mut Criterion) {
    let mut group = c.benchmark_group("pool_allocator");
    
    for block_size in [64, 256, 1024].iter() {
        for count in [1000, 10000].iter() {
            group.throughput(Throughput::Elements(*count as u64));
            group.bench_with_input(
                BenchmarkId::new(format!("{}B", block_size), count),
                &(*block_size, *count),
                |b, &(block_size, count)| {
                    b.iter(|| {
                        let layout = Layout::from_size_align(block_size, 8).unwrap();
                        let mut ptrs = Vec::with_capacity(count);
                        
                        // Allocate
                        for _ in 0..count {
                            let ptr = unsafe { alloc(layout) };
                            ptrs.push(ptr);
                        }
                        
                        // Deallocate
                        for ptr in ptrs {
                            unsafe { dealloc(ptr, layout) };
                        }
                    });
                },
            );
        }
    }
    
    group.finish();
}

fn bench_arena_allocator(c: &mut Criterion) {
    let mut group = c.benchmark_group("arena_allocator");
    
    // Simulate arena allocation pattern
    for total_size in [1024 * 1024, 16 * 1024 * 1024].iter() {
        for alloc_size in [64, 256, 1024].iter() {
            let count = total_size / alloc_size;
            
            group.throughput(Throughput::Bytes(*total_size as u64));
            group.bench_with_input(
                BenchmarkId::new(format!("{}KB_{}B", total_size / 1024, alloc_size), count),
                &(*alloc_size, count),
                |b, &(alloc_size, count)| {
                    b.iter(|| {
                        // Simulate arena: single large allocation
                        let layout = Layout::from_size_align(alloc_size * count, 8).unwrap();
                        let arena = unsafe { alloc(layout) };
                        
                        // Bump allocate within arena
                        let mut offset = 0;
                        let mut ptrs = Vec::with_capacity(count);
                        for _ in 0..count {
                            let ptr = unsafe { arena.add(offset) };
                            ptrs.push(ptr);
                            offset += alloc_size;
                        }
                        
                        black_box(&ptrs);
                        
                        // Single deallocation
                        unsafe { dealloc(arena, layout) };
                    });
                },
            );
        }
    }
    
    group.finish();
}

fn bench_ring_buffer(c: &mut Criterion) {
    let mut group = c.benchmark_group("ring_buffer");
    
    for capacity in [1024, 4096, 16384].iter() {
        group.throughput(Throughput::Elements(*capacity as u64));
        group.bench_with_input(BenchmarkId::from_parameter(capacity), capacity, |b, &capacity| {
            b.iter(|| {
                let mut buffer: Vec<u32> = Vec::with_capacity(capacity);
                let mut head = 0;
                let mut tail = 0;
                
                // Fill buffer
                for i in 0..capacity {
                    buffer.push(i as u32);
                    tail = (tail + 1) % capacity;
                }
                
                // Simulate read/write cycles
                for i in 0..capacity {
                    let _ = buffer[head];
                    head = (head + 1) % capacity;
                    
                    buffer[tail] = i as u32;
                    tail = (tail + 1) % capacity;
                }
                
                black_box(&buffer)
            });
        });
    }
    
    group.finish();
}

fn bench_memory_copy(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_copy");
    
    for size in [1024, 64 * 1024, 1024 * 1024, 16 * 1024 * 1024].iter() {
        let src: Vec<u8> = (0..*size).map(|i| (i % 256) as u8).collect();
        let mut dst: Vec<u8> = vec![0; *size];
        
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                dst.copy_from_slice(&src);
                black_box(&dst)
            });
        });
    }
    
    group.finish();
}

fn bench_memory_set(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_set");
    
    for size in [1024, 64 * 1024, 1024 * 1024, 16 * 1024 * 1024].iter() {
        let mut buffer: Vec<u8> = vec![0; *size];
        
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                buffer.fill(0xFF);
                black_box(&buffer)
            });
        });
    }
    
    group.finish();
}

fn bench_cache_line_access(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_line_access");
    
    const SIZE: usize = 64 * 1024 * 1024; // 64 MB
    let data: Vec<u64> = vec![1; SIZE / 8];
    
    // Sequential access
    group.bench_function("sequential", |b| {
        b.iter(|| {
            let mut sum: u64 = 0;
            for &val in &data {
                sum = sum.wrapping_add(val);
            }
            black_box(sum)
        });
    });
    
    // Strided access (cache-unfriendly)
    for stride in [8, 64, 512].iter() {
        group.bench_with_input(BenchmarkId::new("strided", stride), stride, |b, &stride| {
            b.iter(|| {
                let mut sum: u64 = 0;
                let mut i = 0;
                while i < data.len() {
                    sum = sum.wrapping_add(data[i]);
                    i += stride;
                }
                black_box(sum)
            });
        });
    }
    
    group.finish();
}

fn bench_allocation_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("allocation_patterns");
    
    // LIFO pattern (stack-like)
    group.bench_function("lifo_1000", |b| {
        b.iter(|| {
            let mut stack: Vec<Box<[u8; 256]>> = Vec::with_capacity(1000);
            
            for _ in 0..1000 {
                stack.push(Box::new([0u8; 256]));
            }
            
            while let Some(item) = stack.pop() {
                black_box(item);
            }
        });
    });
    
    // FIFO pattern (queue-like)
    group.bench_function("fifo_1000", |b| {
        b.iter(|| {
            let mut queue: std::collections::VecDeque<Box<[u8; 256]>> = 
                std::collections::VecDeque::with_capacity(1000);
            
            for _ in 0..1000 {
                queue.push_back(Box::new([0u8; 256]));
            }
            
            while let Some(item) = queue.pop_front() {
                black_box(item);
            }
        });
    });
    
    // Random pattern
    group.bench_function("random_1000", |b| {
        b.iter(|| {
            let mut items: Vec<Option<Box<[u8; 256]>>> = (0..1000)
                .map(|_| Some(Box::new([0u8; 256])))
                .collect();
            
            // Pseudo-random deallocation
            for i in (0..1000).step_by(3) {
                items[i] = None;
            }
            for i in (1..1000).step_by(3) {
                items[i] = None;
            }
            for i in (2..1000).step_by(3) {
                items[i] = None;
            }
            
            black_box(&items)
        });
    });
    
    group.finish();
}

fn bench_fragmentation_resistance(c: &mut Criterion) {
    let mut group = c.benchmark_group("fragmentation_resistance");
    
    // Simulate fragmentation scenario
    group.bench_function("mixed_sizes", |b| {
        b.iter(|| {
            let mut allocations: Vec<Vec<u8>> = Vec::new();
            
            // Allocate mixed sizes
            for i in 0..500 {
                let size = match i % 4 {
                    0 => 64,
                    1 => 256,
                    2 => 1024,
                    _ => 4096,
                };
                allocations.push(vec![0u8; size]);
            }
            
            // Free every other allocation
            for i in (0..allocations.len()).rev().step_by(2) {
                allocations.remove(i);
            }
            
            // Allocate again
            for i in 0..250 {
                let size = match i % 4 {
                    0 => 128,
                    1 => 512,
                    2 => 2048,
                    _ => 8192,
                };
                allocations.push(vec![0u8; size]);
            }
            
            black_box(&allocations)
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    bench_pool_allocator,
    bench_arena_allocator,
    bench_ring_buffer,
    bench_memory_copy,
    bench_memory_set,
    bench_cache_line_access,
    bench_allocation_patterns,
    bench_fragmentation_resistance,
);

criterion_main!(benches);
