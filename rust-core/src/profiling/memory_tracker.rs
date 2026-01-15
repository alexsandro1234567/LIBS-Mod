//! # Memory Tracker
//! 
//! Memory allocation tracking and leak detection.

use std::collections::HashMap;

/// Memory tracker
pub struct MemoryTracker {
    /// Allocations by category
    categories: HashMap<String, CategoryStats>,
    /// Total allocated bytes
    total_allocated: usize,
    /// Peak allocated bytes
    peak_allocated: usize,
    /// Allocation count
    allocation_count: u64,
    /// Deallocation count
    deallocation_count: u64,
}

impl MemoryTracker {
    /// Create a new memory tracker
    pub fn new() -> Self {
        Self {
            categories: HashMap::new(),
            total_allocated: 0,
            peak_allocated: 0,
            allocation_count: 0,
            deallocation_count: 0,
        }
    }
    
    /// Track an allocation
    pub fn allocate(&mut self, category: &str, size: usize) {
        let stats = self.categories.entry(category.to_string())
            .or_insert_with(CategoryStats::new);
        
        stats.allocate(size);
        self.total_allocated += size;
        self.peak_allocated = self.peak_allocated.max(self.total_allocated);
        self.allocation_count += 1;
    }
    
    /// Track a deallocation
    pub fn deallocate(&mut self, category: &str, size: usize) {
        if let Some(stats) = self.categories.get_mut(category) {
            stats.deallocate(size);
        }
        
        self.total_allocated = self.total_allocated.saturating_sub(size);
        self.deallocation_count += 1;
    }
    
    /// Get memory statistics
    pub fn stats(&self) -> MemoryStats {
        let category_stats: HashMap<String, CategoryMemoryStats> = self.categories.iter()
            .map(|(name, stats)| (name.clone(), stats.to_stats()))
            .collect();
        
        MemoryStats {
            total_allocated: self.total_allocated,
            peak_allocated: self.peak_allocated,
            allocation_count: self.allocation_count,
            deallocation_count: self.deallocation_count,
            categories: category_stats,
        }
    }
    
    /// Get category statistics
    pub fn category_stats(&self, category: &str) -> Option<CategoryMemoryStats> {
        self.categories.get(category).map(|s| s.to_stats())
    }
    
    /// Get all category names
    pub fn categories(&self) -> Vec<String> {
        self.categories.keys().cloned().collect()
    }
    
    /// Reset tracker
    pub fn reset(&mut self) {
        self.categories.clear();
        self.total_allocated = 0;
        self.peak_allocated = 0;
        self.allocation_count = 0;
        self.deallocation_count = 0;
    }
    
    /// Check for potential leaks
    pub fn check_leaks(&self) -> Vec<LeakReport> {
        let mut leaks = Vec::new();
        
        for (name, stats) in &self.categories {
            if stats.current_allocated > 0 && stats.allocation_count > stats.deallocation_count {
                leaks.push(LeakReport {
                    category: name.clone(),
                    leaked_bytes: stats.current_allocated,
                    leaked_allocations: stats.allocation_count - stats.deallocation_count,
                });
            }
        }
        
        leaks
    }
}

impl Default for MemoryTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Per-category statistics
struct CategoryStats {
    current_allocated: usize,
    peak_allocated: usize,
    total_allocated: usize,
    allocation_count: u64,
    deallocation_count: u64,
}

impl CategoryStats {
    fn new() -> Self {
        Self {
            current_allocated: 0,
            peak_allocated: 0,
            total_allocated: 0,
            allocation_count: 0,
            deallocation_count: 0,
        }
    }
    
    fn allocate(&mut self, size: usize) {
        self.current_allocated += size;
        self.total_allocated += size;
        self.peak_allocated = self.peak_allocated.max(self.current_allocated);
        self.allocation_count += 1;
    }
    
    fn deallocate(&mut self, size: usize) {
        self.current_allocated = self.current_allocated.saturating_sub(size);
        self.deallocation_count += 1;
    }
    
    fn to_stats(&self) -> CategoryMemoryStats {
        CategoryMemoryStats {
            current_allocated: self.current_allocated,
            peak_allocated: self.peak_allocated,
            total_allocated: self.total_allocated,
            allocation_count: self.allocation_count,
            deallocation_count: self.deallocation_count,
        }
    }
}

/// Memory statistics
#[derive(Debug, Clone, Default)]
pub struct MemoryStats {
    pub total_allocated: usize,
    pub peak_allocated: usize,
    pub allocation_count: u64,
    pub deallocation_count: u64,
    pub categories: HashMap<String, CategoryMemoryStats>,
}

impl MemoryStats {
    /// Format as human-readable string
    pub fn format(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!("Total Allocated: {}\n", format_bytes(self.total_allocated)));
        s.push_str(&format!("Peak Allocated:  {}\n", format_bytes(self.peak_allocated)));
        s.push_str(&format!("Allocations:     {}\n", self.allocation_count));
        s.push_str(&format!("Deallocations:   {}\n", self.deallocation_count));
        s.push_str("\nCategories:\n");
        
        for (name, stats) in &self.categories {
            s.push_str(&format!("  {}: {} (peak: {})\n", 
                name, 
                format_bytes(stats.current_allocated),
                format_bytes(stats.peak_allocated)
            ));
        }
        
        s
    }
}

/// Category memory statistics
#[derive(Debug, Clone, Default)]
pub struct CategoryMemoryStats {
    pub current_allocated: usize,
    pub peak_allocated: usize,
    pub total_allocated: usize,
    pub allocation_count: u64,
    pub deallocation_count: u64,
}

/// Leak report
#[derive(Debug, Clone)]
pub struct LeakReport {
    pub category: String,
    pub leaked_bytes: usize,
    pub leaked_allocations: u64,
}

/// Format bytes as human-readable string
pub fn format_bytes(bytes: usize) -> String {
    const KB: usize = 1024;
    const MB: usize = KB * 1024;
    const GB: usize = MB * 1024;
    
    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Memory category constants
pub mod categories {
    pub const TEXTURES: &str = "textures";
    pub const MESHES: &str = "meshes";
    pub const BUFFERS: &str = "buffers";
    pub const SHADERS: &str = "shaders";
    pub const AUDIO: &str = "audio";
    pub const PARTICLES: &str = "particles";
    pub const ENTITIES: &str = "entities";
    pub const CHUNKS: &str = "chunks";
    pub const NETWORK: &str = "network";
    pub const MISC: &str = "misc";
}

/// VRAM tracker for GPU memory
pub struct VramTracker {
    /// Allocations by type
    allocations: HashMap<String, VramAllocation>,
    /// Total VRAM used
    total_used: u64,
    /// VRAM budget (if known)
    budget: Option<u64>,
}

impl VramTracker {
    /// Create a new VRAM tracker
    pub fn new(budget: Option<u64>) -> Self {
        Self {
            allocations: HashMap::new(),
            total_used: 0,
            budget,
        }
    }
    
    /// Track VRAM allocation
    pub fn allocate(&mut self, name: &str, size: u64, allocation_type: VramAllocationType) {
        self.allocations.insert(name.to_string(), VramAllocation {
            size,
            allocation_type,
        });
        self.total_used += size;
    }
    
    /// Track VRAM deallocation
    pub fn deallocate(&mut self, name: &str) {
        if let Some(alloc) = self.allocations.remove(name) {
            self.total_used = self.total_used.saturating_sub(alloc.size);
        }
    }
    
    /// Get total VRAM used
    pub fn total_used(&self) -> u64 {
        self.total_used
    }
    
    /// Get VRAM budget
    pub fn budget(&self) -> Option<u64> {
        self.budget
    }
    
    /// Get usage percentage
    pub fn usage_percentage(&self) -> Option<f64> {
        self.budget.map(|b| self.total_used as f64 / b as f64 * 100.0)
    }
    
    /// Get allocations by type
    pub fn by_type(&self, allocation_type: VramAllocationType) -> u64 {
        self.allocations.values()
            .filter(|a| a.allocation_type == allocation_type)
            .map(|a| a.size)
            .sum()
    }
}

impl Default for VramTracker {
    fn default() -> Self {
        Self::new(None)
    }
}

/// VRAM allocation
struct VramAllocation {
    size: u64,
    allocation_type: VramAllocationType,
}

/// VRAM allocation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VramAllocationType {
    Texture,
    Buffer,
    RenderTarget,
    DepthBuffer,
    Staging,
    Other,
}
