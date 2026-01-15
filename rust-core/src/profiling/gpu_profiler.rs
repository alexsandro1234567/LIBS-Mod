//! # GPU Profiler
//! 
//! Vulkan GPU timing and profiling using timestamp queries.

use std::collections::HashMap;
use ash::vk;

/// GPU profiler using Vulkan timestamp queries
pub struct GpuProfiler {
    /// Query pool for timestamps
    query_pool: vk::QueryPool,
    /// Number of queries in pool
    query_count: u32,
    /// Current query index
    current_query: u32,
    /// Named query ranges
    ranges: HashMap<String, QueryRange>,
    /// Timestamp period (nanoseconds per tick)
    timestamp_period: f32,
    /// Results buffer
    results: Vec<u64>,
    /// Is profiling enabled
    enabled: bool,
}

/// Query range (start/end pair)
#[derive(Debug, Clone)]
struct QueryRange {
    start_query: u32,
    end_query: u32,
    depth: u32,
}

impl GpuProfiler {
    /// Create a new GPU profiler
    pub fn new(
        device: &ash::Device,
        timestamp_period: f32,
        max_queries: u32,
    ) -> Result<Self, vk::Result> {
        let pool_info = vk::QueryPoolCreateInfo::default()
            .query_type(vk::QueryType::TIMESTAMP)
            .query_count(max_queries);
        
        let query_pool = unsafe { device.create_query_pool(&pool_info, None)? };
        
        Ok(Self {
            query_pool,
            query_count: max_queries,
            current_query: 0,
            ranges: HashMap::new(),
            timestamp_period,
            results: vec![0; max_queries as usize],
            enabled: true,
        })
    }
    
    /// Enable/disable profiling
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
    
    /// Reset queries for new frame
    pub fn begin_frame(&mut self, device: &ash::Device, cmd: vk::CommandBuffer) {
        if !self.enabled {
            return;
        }
        
        // Reset query pool
        unsafe {
            device.cmd_reset_query_pool(cmd, self.query_pool, 0, self.query_count);
        }
        
        self.current_query = 0;
        self.ranges.clear();
    }
    
    /// End frame and collect results
    pub fn end_frame(&mut self, device: &ash::Device) -> HashMap<String, GpuTimingResult> {
        if !self.enabled || self.current_query == 0 {
            return HashMap::new();
        }
        
        // Get query results
        let result = unsafe {
            device.get_query_pool_results(
                self.query_pool,
                0,
                &mut self.results[..self.current_query as usize],
                vk::QueryResultFlags::TYPE_64 | vk::QueryResultFlags::WAIT,
            )
        };
        
        if result.is_err() {
            return HashMap::new();
        }
        
        // Calculate timings
        let mut timings = HashMap::new();
        
        for (name, range) in &self.ranges {
            let start = self.results[range.start_query as usize];
            let end = self.results[range.end_query as usize];
            
            let duration_ns = (end - start) as f64 * self.timestamp_period as f64;
            let duration_ms = duration_ns / 1_000_000.0;
            
            timings.insert(name.clone(), GpuTimingResult {
                name: name.clone(),
                duration_ns,
                duration_ms,
                depth: range.depth,
            });
        }
        
        timings
    }
    
    /// Begin a named GPU timing region
    pub fn begin_region(&mut self, device: &ash::Device, cmd: vk::CommandBuffer, name: &str, depth: u32) {
        if !self.enabled || self.current_query >= self.query_count - 1 {
            return;
        }
        
        let start_query = self.current_query;
        self.current_query += 1;
        
        unsafe {
            device.cmd_write_timestamp(
                cmd,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                self.query_pool,
                start_query,
            );
        }
        
        // Store range (end will be filled in end_region)
        self.ranges.insert(name.to_string(), QueryRange {
            start_query,
            end_query: 0,
            depth,
        });
    }
    
    /// End a named GPU timing region
    pub fn end_region(&mut self, device: &ash::Device, cmd: vk::CommandBuffer, name: &str) {
        if !self.enabled || self.current_query >= self.query_count {
            return;
        }
        
        let end_query = self.current_query;
        self.current_query += 1;
        
        unsafe {
            device.cmd_write_timestamp(
                cmd,
                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                self.query_pool,
                end_query,
            );
        }
        
        // Update range with end query
        if let Some(range) = self.ranges.get_mut(name) {
            range.end_query = end_query;
        }
    }
    
    /// Get query pool handle
    pub fn query_pool(&self) -> vk::QueryPool {
        self.query_pool
    }
    
    /// Destroy the profiler
    pub fn destroy(&mut self, device: &ash::Device) {
        unsafe {
            device.destroy_query_pool(self.query_pool, None);
        }
    }
}

/// GPU timing result
#[derive(Debug, Clone)]
pub struct GpuTimingResult {
    pub name: String,
    pub duration_ns: f64,
    pub duration_ms: f64,
    pub depth: u32,
}

/// GPU statistics collector
pub struct GpuStats {
    /// Frame GPU times
    frame_times: Vec<f64>,
    /// Maximum history
    max_history: usize,
    /// Per-region statistics
    region_stats: HashMap<String, RegionStats>,
}

impl GpuStats {
    /// Create new GPU stats collector
    pub fn new(max_history: usize) -> Self {
        Self {
            frame_times: Vec::new(),
            max_history,
            region_stats: HashMap::new(),
        }
    }
    
    /// Record frame timings
    pub fn record_frame(&mut self, timings: &HashMap<String, GpuTimingResult>) {
        // Calculate total frame GPU time
        let total: f64 = timings.values()
            .filter(|t| t.depth == 0)
            .map(|t| t.duration_ms)
            .sum();
        
        self.frame_times.push(total);
        if self.frame_times.len() > self.max_history {
            self.frame_times.remove(0);
        }
        
        // Update per-region stats
        for (name, timing) in timings {
            let stats = self.region_stats.entry(name.clone()).or_insert_with(RegionStats::new);
            stats.record(timing.duration_ms);
        }
    }
    
    /// Get average GPU frame time
    pub fn average_frame_time(&self) -> f64 {
        if self.frame_times.is_empty() {
            return 0.0;
        }
        self.frame_times.iter().sum::<f64>() / self.frame_times.len() as f64
    }
    
    /// Get region statistics
    pub fn region_stats(&self, name: &str) -> Option<&RegionStats> {
        self.region_stats.get(name)
    }
    
    /// Get all region names
    pub fn region_names(&self) -> Vec<String> {
        self.region_stats.keys().cloned().collect()
    }
    
    /// Reset statistics
    pub fn reset(&mut self) {
        self.frame_times.clear();
        self.region_stats.clear();
    }
}

impl Default for GpuStats {
    fn default() -> Self {
        Self::new(300)
    }
}

/// Per-region statistics
#[derive(Debug, Clone)]
pub struct RegionStats {
    pub min_ms: f64,
    pub max_ms: f64,
    pub avg_ms: f64,
    pub total_ms: f64,
    pub count: u64,
    history: Vec<f64>,
    max_history: usize,
}

impl RegionStats {
    pub fn new() -> Self {
        Self {
            min_ms: f64::INFINITY,
            max_ms: f64::NEG_INFINITY,
            avg_ms: 0.0,
            total_ms: 0.0,
            count: 0,
            history: Vec::new(),
            max_history: 100,
        }
    }
    
    pub fn record(&mut self, duration_ms: f64) {
        self.min_ms = self.min_ms.min(duration_ms);
        self.max_ms = self.max_ms.max(duration_ms);
        self.total_ms += duration_ms;
        self.count += 1;
        self.avg_ms = self.total_ms / self.count as f64;
        
        self.history.push(duration_ms);
        if self.history.len() > self.max_history {
            self.history.remove(0);
        }
    }
    
    pub fn recent_average(&self) -> f64 {
        if self.history.is_empty() {
            return 0.0;
        }
        self.history.iter().sum::<f64>() / self.history.len() as f64
    }
}

impl Default for RegionStats {
    fn default() -> Self {
        Self::new()
    }
}
