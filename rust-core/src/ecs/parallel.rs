//! # Parallel ECS Execution
//! 
//! Hyper-threaded entity processing with automatic dependency detection.
//! Distributes entity ticks across multiple CPU cores.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use parking_lot::{RwLock, Mutex};
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};

/// Entity ID type
pub type EntityId = u64;

/// Chunk position for grouping
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl ChunkPos {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
    
    pub fn from_world_pos(x: f64, y: f64, z: f64) -> Self {
        Self {
            x: (x / 16.0).floor() as i32,
            y: (y / 16.0).floor() as i32,
            z: (z / 16.0).floor() as i32,
        }
    }
}

/// Entity dependency flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DependencyFlags {
    /// Entity reads from other entities
    pub reads_entities: bool,
    /// Entity writes to other entities
    pub writes_entities: bool,
    /// Entity reads from world
    pub reads_world: bool,
    /// Entity writes to world
    pub writes_world: bool,
    /// Entity uses networking
    pub uses_network: bool,
}

impl Default for DependencyFlags {
    fn default() -> Self {
        Self {
            reads_entities: false,
            writes_entities: false,
            reads_world: true,
            writes_world: false,
            uses_network: false,
        }
    }
}

impl DependencyFlags {
    /// Check if entity is independent (can run in parallel)
    pub fn is_independent(&self) -> bool {
        !self.writes_entities && !self.writes_world
    }
    
    /// Check if entity conflicts with another
    pub fn conflicts_with(&self, other: &DependencyFlags) -> bool {
        // Write-write conflict
        if self.writes_entities && other.writes_entities {
            return true;
        }
        // Read-write conflict
        if (self.reads_entities && other.writes_entities) ||
           (self.writes_entities && other.reads_entities) {
            return true;
        }
        // World write conflict
        if self.writes_world && other.writes_world {
            return true;
        }
        false
    }
}

/// Entity tick data for parallel processing
#[derive(Debug, Clone)]
pub struct EntityTickData {
    pub entity_id: EntityId,
    pub chunk: ChunkPos,
    pub dependencies: DependencyFlags,
    pub priority: i32,
    pub last_tick_time_ns: u64,
}

/// Chunk group for parallel processing
#[derive(Debug)]
pub struct ChunkGroup {
    pub chunk: ChunkPos,
    pub entities: Vec<EntityId>,
    pub can_parallel: bool,
}

/// Tick batch that can run in parallel
#[derive(Debug)]
pub struct TickBatch {
    pub batch_id: u32,
    pub chunks: Vec<ChunkPos>,
    pub entity_count: usize,
}

/// Parallel tick scheduler
pub struct ParallelScheduler {
    /// Entity data map
    entities: HashMap<EntityId, EntityTickData>,
    /// Chunk groupings
    chunk_groups: HashMap<ChunkPos, ChunkGroup>,
    /// Tick batches for parallel execution
    tick_batches: Vec<TickBatch>,
    /// Entity independence detector
    independence_cache: HashMap<EntityId, bool>,
    /// Thread pool size
    thread_count: usize,
    /// Stats
    stats: ParallelStats,
    /// Initialized flag
    initialized: bool,
}

/// Parallel execution statistics
#[derive(Debug, Default, Clone)]
pub struct ParallelStats {
    pub total_entities: u64,
    pub parallel_entities: u64,
    pub sequential_entities: u64,
    pub batch_count: u32,
    pub avg_batch_size: f32,
    pub last_tick_time_us: u64,
}

impl ParallelScheduler {
    pub fn new() -> Self {
        let thread_count = num_cpus::get().saturating_sub(2).max(1);
        
        log::info!("Creating ParallelScheduler with {} worker threads", thread_count);
        
        Self {
            entities: HashMap::with_capacity(10000),
            chunk_groups: HashMap::with_capacity(1000),
            tick_batches: Vec::with_capacity(64),
            independence_cache: HashMap::with_capacity(10000),
            thread_count,
            stats: ParallelStats::default(),
            initialized: false,
        }
    }
    
    /// Initialize the scheduler
    pub fn initialize(&mut self) {
        // Initialize rayon thread pool
        rayon::ThreadPoolBuilder::new()
            .num_threads(self.thread_count)
            .thread_name(|i| format!("libs-ecs-{}", i))
            .build_global()
            .ok();
        
        self.initialized = true;
        log::info!("ParallelScheduler initialized");
    }
    
    /// Register entity for parallel ticking
    pub fn register_entity(&mut self, entity_id: EntityId, chunk: ChunkPos, dependencies: DependencyFlags) {
        let tick_data = EntityTickData {
            entity_id,
            chunk,
            dependencies,
            priority: 0,
            last_tick_time_ns: 0,
        };
        
        // Check independence
        let is_independent = dependencies.is_independent();
        self.independence_cache.insert(entity_id, is_independent);
        
        // Add to chunk group
        let group = self.chunk_groups.entry(chunk).or_insert_with(|| ChunkGroup {
            chunk,
            entities: Vec::new(),
            can_parallel: true,
        });
        group.entities.push(entity_id);
        
        // Update group parallelism
        if !is_independent {
            group.can_parallel = false;
        }
        
        self.entities.insert(entity_id, tick_data);
    }
    
    /// Unregister entity
    pub fn unregister_entity(&mut self, entity_id: EntityId) {
        if let Some(data) = self.entities.remove(&entity_id) {
            if let Some(group) = self.chunk_groups.get_mut(&data.chunk) {
                group.entities.retain(|&id| id != entity_id);
            }
        }
        self.independence_cache.remove(&entity_id);
    }
    
    /// Update entity chunk position
    pub fn update_entity_chunk(&mut self, entity_id: EntityId, new_chunk: ChunkPos) {
        if let Some(data) = self.entities.get_mut(&entity_id) {
            let old_chunk = data.chunk;
            if old_chunk != new_chunk {
                // Remove from old chunk
                if let Some(group) = self.chunk_groups.get_mut(&old_chunk) {
                    group.entities.retain(|&id| id != entity_id);
                }
                
                // Add to new chunk
                let group = self.chunk_groups.entry(new_chunk).or_insert_with(|| ChunkGroup {
                    chunk: new_chunk,
                    entities: Vec::new(),
                    can_parallel: true,
                });
                group.entities.push(entity_id);
                
                data.chunk = new_chunk;
            }
        }
    }
    
    /// Detect entity independence automatically
    pub fn detect_independence(&mut self, entity_id: EntityId) -> bool {
        if let Some(&cached) = self.independence_cache.get(&entity_id) {
            return cached;
        }
        
        // Default to independent if not registered
        true
    }
    
    /// Build tick batches for parallel execution
    pub fn build_batches(&mut self) {
        self.tick_batches.clear();
        
        // Separate chunks into parallelizable and sequential
        let mut parallel_chunks: Vec<ChunkPos> = Vec::new();
        let mut sequential_chunks: Vec<ChunkPos> = Vec::new();
        
        for (chunk, group) in &self.chunk_groups {
            if group.entities.is_empty() {
                continue;
            }
            
            if group.can_parallel {
                parallel_chunks.push(*chunk);
            } else {
                sequential_chunks.push(*chunk);
            }
        }
        
        // Create batches for parallel chunks
        // Group nearby chunks into same batch for cache locality
        let chunks_per_batch = (parallel_chunks.len() / self.thread_count).max(1);
        
        for (batch_id, chunk_batch) in parallel_chunks.chunks(chunks_per_batch).enumerate() {
            let entity_count: usize = chunk_batch.iter()
                .filter_map(|c| self.chunk_groups.get(c))
                .map(|g| g.entities.len())
                .sum();
            
            self.tick_batches.push(TickBatch {
                batch_id: batch_id as u32,
                chunks: chunk_batch.to_vec(),
                entity_count,
            });
        }
        
        // Add sequential batch at the end
        if !sequential_chunks.is_empty() {
            let entity_count: usize = sequential_chunks.iter()
                .filter_map(|c| self.chunk_groups.get(c))
                .map(|g| g.entities.len())
                .sum();
            
            self.tick_batches.push(TickBatch {
                batch_id: self.tick_batches.len() as u32,
                chunks: sequential_chunks,
                entity_count,
            });
        }
        
        // Update stats
        self.stats.batch_count = self.tick_batches.len() as u32;
        if !self.tick_batches.is_empty() {
            self.stats.avg_batch_size = self.entities.len() as f32 / self.tick_batches.len() as f32;
        }
    }
    
    /// Execute parallel tick with callback
    pub fn parallel_tick<F>(&mut self, delta_time: f32, tick_fn: F)
    where
        F: Fn(EntityId, f32) + Send + Sync,
    {
        let start = std::time::Instant::now();
        
        // Rebuild batches if needed
        self.build_batches();
        
        let mut parallel_count = 0u64;
        let mut sequential_count = 0u64;
        
        // Process parallel batches
        for batch in &self.tick_batches {
            let batch_entities: Vec<EntityId> = batch.chunks.iter()
                .filter_map(|c| self.chunk_groups.get(c))
                .flat_map(|g| g.entities.iter().copied())
                .collect();
            
            // Check if this batch can run in parallel
            let can_parallel = batch.chunks.iter()
                .filter_map(|c| self.chunk_groups.get(c))
                .all(|g| g.can_parallel);
            
            if can_parallel && batch_entities.len() > 1 {
                // Parallel execution
                batch_entities.par_iter().for_each(|&entity_id| {
                    tick_fn(entity_id, delta_time);
                });
                parallel_count += batch_entities.len() as u64;
            } else {
                // Sequential execution
                for entity_id in batch_entities {
                    tick_fn(entity_id, delta_time);
                    sequential_count += 1;
                }
            }
        }
        
        // Update stats
        self.stats.total_entities = parallel_count + sequential_count;
        self.stats.parallel_entities = parallel_count;
        self.stats.sequential_entities = sequential_count;
        self.stats.last_tick_time_us = start.elapsed().as_micros() as u64;
    }
    
    /// Get statistics
    pub fn stats(&self) -> &ParallelStats {
        &self.stats
    }
    
    /// Get entity count
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }
    
    /// Get chunk count
    pub fn chunk_count(&self) -> usize {
        self.chunk_groups.len()
    }
    
    /// Clear all entities
    pub fn clear(&mut self) {
        self.entities.clear();
        self.chunk_groups.clear();
        self.tick_batches.clear();
        self.independence_cache.clear();
        self.stats = ParallelStats::default();
    }
    
    /// Shutdown
    pub fn shutdown(&mut self) {
        self.clear();
        self.initialized = false;
        log::info!("ParallelScheduler shutdown");
    }
}

impl Default for ParallelScheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_scheduler_creation() {
        let scheduler = ParallelScheduler::new();
        assert!(scheduler.thread_count >= 1);
    }
    
    #[test]
    fn test_entity_registration() {
        let mut scheduler = ParallelScheduler::new();
        
        scheduler.register_entity(
            1,
            ChunkPos::new(0, 0, 0),
            DependencyFlags::default(),
        );
        
        assert_eq!(scheduler.entity_count(), 1);
        assert_eq!(scheduler.chunk_count(), 1);
    }
    
    #[test]
    fn test_independence_detection() {
        let deps = DependencyFlags {
            reads_entities: true,
            writes_entities: false,
            reads_world: true,
            writes_world: false,
            uses_network: false,
        };
        
        assert!(deps.is_independent());
        
        let dependent = DependencyFlags {
            writes_entities: true,
            ..Default::default()
        };
        
        assert!(!dependent.is_independent());
    }
}
