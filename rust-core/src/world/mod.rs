//! # World Management Module
//! 
//! Chunk storage and world data management.

pub mod assets;

pub use assets::NbtAssetLoader;

use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};

/// Next chunk handle
static NEXT_CHUNK_HANDLE: AtomicI64 = AtomicI64::new(1);

/// World manager
pub struct WorldManager {
    /// Loaded chunks by (x, z) key
    chunks: HashMap<(i32, i32), ChunkData>,
    
    /// Chunk handles mapping
    chunk_handles: HashMap<i64, (i32, i32)>,
    
    /// Dirty chunks that need re-meshing
    dirty_chunks: Vec<(i32, i32)>,
}

/// Chunk data container
pub struct ChunkData {
    /// Chunk handle
    handle: i64,
    
    /// Chunk X coordinate
    x: i32,
    
    /// Chunk Z coordinate
    z: i32,
    
    /// Block data (sections)
    sections: Vec<ChunkSection>,
    
    /// Is meshed
    meshed: bool,
    
    /// Is dirty (needs re-mesh)
    dirty: bool,
    
    /// Raw data for mesh generation
    raw_data: Vec<u8>,
}

/// A 16x16x16 chunk section
pub struct ChunkSection {
    /// Section Y index
    y: i32,
    
    /// Block IDs (4096 entries)
    blocks: Vec<u16>,
    
    /// Light levels
    light: Vec<u8>,
    
    /// Is empty (all air)
    empty: bool,
}

impl ChunkSection {
    /// Create a new empty section
    pub fn new(y: i32) -> Self {
        Self {
            y,
            blocks: vec![0; 4096],
            light: vec![0; 4096],
            empty: true,
        }
    }
    
    /// Get block at local coordinates
    pub fn get_block(&self, x: usize, y: usize, z: usize) -> u16 {
        let index = (y << 8) | (z << 4) | x;
        self.blocks.get(index).copied().unwrap_or(0)
    }
    
    /// Set block at local coordinates
    pub fn set_block(&mut self, x: usize, y: usize, z: usize, block_id: u16) {
        let index = (y << 8) | (z << 4) | x;
        if index < self.blocks.len() {
            self.blocks[index] = block_id;
            self.empty = block_id == 0 && self.blocks.iter().all(|&b| b == 0);
        }
    }
}

impl WorldManager {
    /// Create a new world manager
    pub fn new() -> Self {
        log::debug!("World manager created");
        
        Self {
            chunks: HashMap::new(),
            chunk_handles: HashMap::new(),
            dirty_chunks: Vec::new(),
        }
    }
    
    /// Process a tick (chunk loading/meshing)
    pub fn tick(&mut self) {
        // Process dirty chunks for meshing
        if !self.dirty_chunks.is_empty() {
            // Process up to 4 chunks per tick
            let to_process: Vec<_> = self.dirty_chunks.drain(..self.dirty_chunks.len().min(4)).collect();
            
            for (x, z) in to_process {
                if let Some(chunk) = self.chunks.get_mut(&(x, z)) {
                    // In full implementation, would generate mesh here
                    chunk.meshed = true;
                    chunk.dirty = false;
                    log::trace!("Chunk ({}, {}) meshed", x, z);
                }
            }
        }
    }
    
    /// Submit chunk data
    pub fn submit_chunk(&mut self, x: i32, z: i32, data: &[u8]) -> i64 {
        let handle = NEXT_CHUNK_HANDLE.fetch_add(1, Ordering::SeqCst);
        
        // Parse chunk data
        let chunk = ChunkData {
            handle,
            x,
            z,
            sections: Vec::new(), // Would parse from data
            meshed: false,
            dirty: true,
            raw_data: data.to_vec(),
        };
        
        self.chunks.insert((x, z), chunk);
        self.chunk_handles.insert(handle, (x, z));
        self.dirty_chunks.push((x, z));
        
        log::trace!("Chunk submitted: ({}, {}) -> handle {}", x, z, handle);
        
        handle
    }
    
    /// Update chunk data
    pub fn update_chunk(&mut self, x: i32, z: i32, data: &[u8]) {
        if let Some(chunk) = self.chunks.get_mut(&(x, z)) {
            chunk.raw_data = data.to_vec();
            chunk.dirty = true;
            chunk.meshed = false;
            
            if !self.dirty_chunks.contains(&(x, z)) {
                self.dirty_chunks.push((x, z));
            }
            
            log::trace!("Chunk updated: ({}, {})", x, z);
        }
    }
    
    /// Mark chunk as dirty
    pub fn mark_chunk_dirty(&mut self, x: i32, z: i32) {
        if let Some(chunk) = self.chunks.get_mut(&(x, z)) {
            chunk.dirty = true;
            chunk.meshed = false;
            
            if !self.dirty_chunks.contains(&(x, z)) {
                self.dirty_chunks.push((x, z));
            }
            
            log::trace!("Chunk marked dirty: ({}, {})", x, z);
        }
    }
    
    /// Unload a chunk
    pub fn unload_chunk(&mut self, x: i32, z: i32) {
        if let Some(chunk) = self.chunks.remove(&(x, z)) {
            self.chunk_handles.remove(&chunk.handle);
            self.dirty_chunks.retain(|&c| c != (x, z));
            log::trace!("Chunk unloaded: ({}, {})", x, z);
        }
    }
    
    /// Set a block
    pub fn set_block(&mut self, x: i32, y: i32, z: i32, block_id: u32) {
        let chunk_x = x >> 4;
        let chunk_z = z >> 4;
        
        if let Some(chunk) = self.chunks.get_mut(&(chunk_x, chunk_z)) {
            // Calculate section and local coordinates
            let section_y = y >> 4;
            let local_x = (x & 15) as usize;
            let local_y = (y & 15) as usize;
            let local_z = (z & 15) as usize;
            
            // Ensure section exists
            while chunk.sections.len() <= section_y as usize {
                chunk.sections.push(ChunkSection::new(chunk.sections.len() as i32));
            }
            
            // Set block
            if let Some(section) = chunk.sections.get_mut(section_y as usize) {
                section.set_block(local_x, local_y, local_z, block_id as u16);
            }
            
            // Mark for re-mesh
            chunk.dirty = true;
            chunk.meshed = false;
            
            if !self.dirty_chunks.contains(&(chunk_x, chunk_z)) {
                self.dirty_chunks.push((chunk_x, chunk_z));
            }
            
            log::trace!("Block set at ({}, {}, {}) = {}", x, y, z, block_id);
        }
    }
    
    /// Get a block
    pub fn get_block(&self, x: i32, y: i32, z: i32) -> u16 {
        let chunk_x = x >> 4;
        let chunk_z = z >> 4;
        
        if let Some(chunk) = self.chunks.get(&(chunk_x, chunk_z)) {
            let section_y = y >> 4;
            let local_x = (x & 15) as usize;
            let local_y = (y & 15) as usize;
            let local_z = (z & 15) as usize;
            
            if let Some(section) = chunk.sections.get(section_y as usize) {
                return section.get_block(local_x, local_y, local_z);
            }
        }
        
        0 // Air
    }
    
    /// Get chunk count
    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }
    
    /// Get dirty chunk count
    pub fn dirty_chunk_count(&self) -> usize {
        self.dirty_chunks.len()
    }
    
    /// Check if chunk is loaded
    pub fn is_chunk_loaded(&self, x: i32, z: i32) -> bool {
        self.chunks.contains_key(&(x, z))
    }
    
    /// Get chunk by coordinates
    pub fn get_chunk(&self, x: i32, z: i32) -> Option<&ChunkData> {
        self.chunks.get(&(x, z))
    }
}

impl Default for WorldManager {
    fn default() -> Self {
        Self::new()
    }
}
