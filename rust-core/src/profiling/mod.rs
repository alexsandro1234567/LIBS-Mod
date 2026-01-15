//! # Profiling and Metrics System
//! 
//! Comprehensive performance monitoring, profiling, and metrics collection.

pub mod timer;
pub mod metrics;
pub mod gpu_profiler;
pub mod memory_tracker;

use std::collections::HashMap;
use std::sync::{Arc, RwLock, atomic::{AtomicU64, AtomicBool, Ordering}};
use std::time::{Duration, Instant};

pub use timer::*;
pub use metrics::*;
pub use gpu_profiler::*;
pub use memory_tracker::*;

/// Global profiler instance
static PROFILER: once_cell::sync::Lazy<Profiler> = once_cell::sync::Lazy::new(Profiler::new);

/// Get global profiler
pub fn profiler() -> &'static Profiler {
    &PROFILER
}

/// Main profiler
pub struct Profiler {
    /// Is profiling enabled
    enabled: AtomicBool,
    /// Frame data
    frames: RwLock<FrameHistory>,
    /// CPU timers
    cpu_timers: RwLock<HashMap<String, TimerData>>,
    /// Metrics collector
    metrics: RwLock<MetricsCollector>,
    /// Memory tracker
    memory: RwLock<MemoryTracker>,
    /// Current frame number
    frame_number: AtomicU64,
    /// Frame start time
    frame_start: RwLock<Instant>,
}

impl Profiler {
    /// Create a new profiler
    pub fn new() -> Self {
        Self {
            enabled: AtomicBool::new(true),
            frames: RwLock::new(FrameHistory::new(300)), // 5 seconds at 60 FPS
            cpu_timers: RwLock::new(HashMap::new()),
            metrics: RwLock::new(MetricsCollector::new()),
            memory: RwLock::new(MemoryTracker::new()),
            frame_number: AtomicU64::new(0),
            frame_start: RwLock::new(Instant::now()),
        }
    }
    
    /// Enable/disable profiling
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::SeqCst);
    }
    
    /// Check if profiling is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }
    
    /// Begin a new frame
    pub fn begin_frame(&self) {
        if !self.is_enabled() {
            return;
        }
        
        *self.frame_start.write().unwrap() = Instant::now();
        self.frame_number.fetch_add(1, Ordering::SeqCst);
    }
    
    /// End the current frame
    pub fn end_frame(&self) {
        if !self.is_enabled() {
            return;
        }
        
        let frame_time = self.frame_start.read().unwrap().elapsed();
        let frame_number = self.frame_number.load(Ordering::SeqCst);
        
        // Collect timer data
        let timers: HashMap<String, Duration> = self.cpu_timers.read().unwrap()
            .iter()
            .map(|(name, data)| (name.clone(), data.last_duration))
            .collect();
        
        // Create frame data
        let frame_data = FrameData {
            frame_number,
            total_time: frame_time,
            cpu_time: frame_time, // Would separate CPU/GPU in full implementation
            gpu_time: Duration::ZERO,
            timers,
            draw_calls: 0,
            triangles: 0,
            vertices: 0,
        };
        
        self.frames.write().unwrap().push(frame_data);
        
        // Update metrics
        self.metrics.write().unwrap().record_frame_time(frame_time);
    }
    
    /// Start a CPU timer
    pub fn start_timer(&self, name: &str) -> TimerGuard {
        TimerGuard::new(name.to_string(), self)
    }
    
    /// Record timer duration
    pub fn record_timer(&self, name: &str, duration: Duration) {
        if !self.is_enabled() {
            return;
        }
        
        let mut timers = self.cpu_timers.write().unwrap();
        let entry = timers.entry(name.to_string()).or_insert_with(TimerData::new);
        entry.record(duration);
    }
    
    /// Record a metric
    pub fn record_metric(&self, name: &str, value: f64) {
        if !self.is_enabled() {
            return;
        }
        
        self.metrics.write().unwrap().record(name, value);
    }
    
    /// Increment a counter
    pub fn increment_counter(&self, name: &str, amount: u64) {
        if !self.is_enabled() {
            return;
        }
        
        self.metrics.write().unwrap().increment(name, amount);
    }
    
    /// Track memory allocation
    pub fn track_allocation(&self, category: &str, size: usize) {
        if !self.is_enabled() {
            return;
        }
        
        self.memory.write().unwrap().allocate(category, size);
    }
    
    /// Track memory deallocation
    pub fn track_deallocation(&self, category: &str, size: usize) {
        if !self.is_enabled() {
            return;
        }
        
        self.memory.write().unwrap().deallocate(category, size);
    }
    
    /// Get frame statistics
    pub fn get_frame_stats(&self) -> FrameStats {
        let frames = self.frames.read().unwrap();
        
        if frames.is_empty() {
            return FrameStats::default();
        }
        
        let frame_times: Vec<f64> = frames.iter()
            .map(|f| f.total_time.as_secs_f64() * 1000.0)
            .collect();
        
        let avg = frame_times.iter().sum::<f64>() / frame_times.len() as f64;
        let min = frame_times.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = frame_times.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        
        // Calculate percentiles
        let mut sorted = frame_times.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let p99 = sorted.get((sorted.len() as f64 * 0.99) as usize).copied().unwrap_or(0.0);
        let p95 = sorted.get((sorted.len() as f64 * 0.95) as usize).copied().unwrap_or(0.0);
        let p50 = sorted.get(sorted.len() / 2).copied().unwrap_or(0.0);
        
        FrameStats {
            fps: 1000.0 / avg,
            avg_frame_time_ms: avg,
            min_frame_time_ms: min,
            max_frame_time_ms: max,
            p99_frame_time_ms: p99,
            p95_frame_time_ms: p95,
            p50_frame_time_ms: p50,
            frame_count: frames.len() as u64,
        }
    }
    
    /// Get timer statistics
    pub fn get_timer_stats(&self, name: &str) -> Option<TimerStats> {
        self.cpu_timers.read().unwrap().get(name).map(|data| data.stats())
    }
    
    /// Get all timer names
    pub fn get_timer_names(&self) -> Vec<String> {
        self.cpu_timers.read().unwrap().keys().cloned().collect()
    }
    
    /// Get memory statistics
    pub fn get_memory_stats(&self) -> MemoryStats {
        self.memory.read().unwrap().stats()
    }
    
    /// Get metric value
    pub fn get_metric(&self, name: &str) -> Option<MetricValue> {
        self.metrics.read().unwrap().get(name)
    }
    
    /// Generate profiling report
    pub fn generate_report(&self) -> ProfilingReport {
        let frame_stats = self.get_frame_stats();
        let memory_stats = self.get_memory_stats();
        
        let timer_stats: HashMap<String, TimerStats> = self.cpu_timers.read().unwrap()
            .iter()
            .map(|(name, data)| (name.clone(), data.stats()))
            .collect();
        
        let metrics = self.metrics.read().unwrap().all_metrics();
        
        ProfilingReport {
            timestamp: chrono::Utc::now(),
            frame_stats,
            memory_stats,
            timer_stats,
            metrics,
        }
    }
    
    /// Reset all profiling data
    pub fn reset(&self) {
        self.frames.write().unwrap().clear();
        self.cpu_timers.write().unwrap().clear();
        self.metrics.write().unwrap().reset();
        self.memory.write().unwrap().reset();
    }
}

impl Default for Profiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Frame history ring buffer
pub struct FrameHistory {
    frames: Vec<FrameData>,
    capacity: usize,
    head: usize,
    count: usize,
}

impl FrameHistory {
    pub fn new(capacity: usize) -> Self {
        Self {
            frames: vec![FrameData::default(); capacity],
            capacity,
            head: 0,
            count: 0,
        }
    }
    
    pub fn push(&mut self, frame: FrameData) {
        self.frames[self.head] = frame;
        self.head = (self.head + 1) % self.capacity;
        if self.count < self.capacity {
            self.count += 1;
        }
    }
    
    pub fn iter(&self) -> impl Iterator<Item = &FrameData> {
        let start = if self.count < self.capacity { 0 } else { self.head };
        (0..self.count).map(move |i| &self.frames[(start + i) % self.capacity])
    }
    
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
    
    pub fn len(&self) -> usize {
        self.count
    }
    
    pub fn clear(&mut self) {
        self.head = 0;
        self.count = 0;
    }
}

/// Frame profiling data
#[derive(Debug, Clone, Default)]
pub struct FrameData {
    pub frame_number: u64,
    pub total_time: Duration,
    pub cpu_time: Duration,
    pub gpu_time: Duration,
    pub timers: HashMap<String, Duration>,
    pub draw_calls: u32,
    pub triangles: u64,
    pub vertices: u64,
}

/// Frame statistics
#[derive(Debug, Clone, Default)]
pub struct FrameStats {
    pub fps: f64,
    pub avg_frame_time_ms: f64,
    pub min_frame_time_ms: f64,
    pub max_frame_time_ms: f64,
    pub p99_frame_time_ms: f64,
    pub p95_frame_time_ms: f64,
    pub p50_frame_time_ms: f64,
    pub frame_count: u64,
}

/// Timer statistics
#[derive(Debug, Clone, Default)]
pub struct TimerStats {
    pub avg_ms: f64,
    pub min_ms: f64,
    pub max_ms: f64,
    pub total_ms: f64,
    pub call_count: u64,
}

/// Timer data
#[derive(Debug, Clone)]
pub struct TimerData {
    pub last_duration: Duration,
    pub total_duration: Duration,
    pub min_duration: Duration,
    pub max_duration: Duration,
    pub call_count: u64,
}

impl TimerData {
    pub fn new() -> Self {
        Self {
            last_duration: Duration::ZERO,
            total_duration: Duration::ZERO,
            min_duration: Duration::MAX,
            max_duration: Duration::ZERO,
            call_count: 0,
        }
    }
    
    pub fn record(&mut self, duration: Duration) {
        self.last_duration = duration;
        self.total_duration += duration;
        self.min_duration = self.min_duration.min(duration);
        self.max_duration = self.max_duration.max(duration);
        self.call_count += 1;
    }
    
    pub fn stats(&self) -> TimerStats {
        let avg = if self.call_count > 0 {
            self.total_duration.as_secs_f64() * 1000.0 / self.call_count as f64
        } else {
            0.0
        };
        
        TimerStats {
            avg_ms: avg,
            min_ms: self.min_duration.as_secs_f64() * 1000.0,
            max_ms: self.max_duration.as_secs_f64() * 1000.0,
            total_ms: self.total_duration.as_secs_f64() * 1000.0,
            call_count: self.call_count,
        }
    }
}

impl Default for TimerData {
    fn default() -> Self {
        Self::new()
    }
}

/// RAII timer guard
pub struct TimerGuard<'a> {
    name: String,
    start: Instant,
    profiler: &'a Profiler,
}

impl<'a> TimerGuard<'a> {
    pub fn new(name: String, profiler: &'a Profiler) -> Self {
        Self {
            name,
            start: Instant::now(),
            profiler,
        }
    }
}

impl<'a> Drop for TimerGuard<'a> {
    fn drop(&mut self) {
        let duration = self.start.elapsed();
        self.profiler.record_timer(&self.name, duration);
    }
}

/// Profiling report
#[derive(Debug, Clone)]
pub struct ProfilingReport {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub frame_stats: FrameStats,
    pub memory_stats: MemoryStats,
    pub timer_stats: HashMap<String, TimerStats>,
    pub metrics: HashMap<String, MetricValue>,
}

impl ProfilingReport {
    /// Export to JSON
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
    
    /// Export to CSV (frame times only)
    pub fn to_csv(&self) -> String {
        let mut csv = String::from("metric,value\n");
        csv.push_str(&format!("fps,{}\n", self.frame_stats.fps));
        csv.push_str(&format!("avg_frame_time_ms,{}\n", self.frame_stats.avg_frame_time_ms));
        csv.push_str(&format!("min_frame_time_ms,{}\n", self.frame_stats.min_frame_time_ms));
        csv.push_str(&format!("max_frame_time_ms,{}\n", self.frame_stats.max_frame_time_ms));
        csv.push_str(&format!("p99_frame_time_ms,{}\n", self.frame_stats.p99_frame_time_ms));
        csv.push_str(&format!("p95_frame_time_ms,{}\n", self.frame_stats.p95_frame_time_ms));
        csv.push_str(&format!("p50_frame_time_ms,{}\n", self.frame_stats.p50_frame_time_ms));
        csv
    }
}

// Implement Serialize for ProfilingReport
impl serde::Serialize for ProfilingReport {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("ProfilingReport", 5)?;
        state.serialize_field("timestamp", &self.timestamp.to_rfc3339())?;
        state.serialize_field("frame_stats", &format!("{:?}", self.frame_stats))?;
        state.serialize_field("memory_stats", &format!("{:?}", self.memory_stats))?;
        state.serialize_field("timer_count", &self.timer_stats.len())?;
        state.serialize_field("metric_count", &self.metrics.len())?;
        state.end()
    }
}

/// Scoped profiler macro
#[macro_export]
macro_rules! profile_scope {
    ($name:expr) => {
        let _guard = $crate::profiling::profiler().start_timer($name);
    };
}

/// Profile function macro
#[macro_export]
macro_rules! profile_function {
    () => {
        let _guard = $crate::profiling::profiler().start_timer(concat!(module_path!(), "::", function_name!()));
    };
}
