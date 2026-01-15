//! # Metrics Collection
//! 
//! Metrics collection and aggregation system.

use std::collections::HashMap;
use std::time::Duration;

/// Metrics collector
pub struct MetricsCollector {
    /// Gauge metrics (current values)
    gauges: HashMap<String, f64>,
    /// Counter metrics (cumulative)
    counters: HashMap<String, u64>,
    /// Histogram metrics
    histograms: HashMap<String, Histogram>,
    /// Frame time history
    frame_times: Vec<Duration>,
    /// Maximum frame time history
    max_frame_history: usize,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            gauges: HashMap::new(),
            counters: HashMap::new(),
            histograms: HashMap::new(),
            frame_times: Vec::new(),
            max_frame_history: 1000,
        }
    }
    
    /// Record a gauge metric
    pub fn record(&mut self, name: &str, value: f64) {
        self.gauges.insert(name.to_string(), value);
    }
    
    /// Increment a counter
    pub fn increment(&mut self, name: &str, amount: u64) {
        *self.counters.entry(name.to_string()).or_insert(0) += amount;
    }
    
    /// Record a histogram value
    pub fn record_histogram(&mut self, name: &str, value: f64) {
        self.histograms
            .entry(name.to_string())
            .or_insert_with(Histogram::new)
            .record(value);
    }
    
    /// Record frame time
    pub fn record_frame_time(&mut self, duration: Duration) {
        self.frame_times.push(duration);
        if self.frame_times.len() > self.max_frame_history {
            self.frame_times.remove(0);
        }
        
        // Update related metrics
        let ms = duration.as_secs_f64() * 1000.0;
        self.record("frame_time_ms", ms);
        self.record("fps", 1000.0 / ms);
        self.record_histogram("frame_time_histogram", ms);
    }
    
    /// Get a metric value
    pub fn get(&self, name: &str) -> Option<MetricValue> {
        if let Some(&value) = self.gauges.get(name) {
            return Some(MetricValue::Gauge(value));
        }
        if let Some(&value) = self.counters.get(name) {
            return Some(MetricValue::Counter(value));
        }
        if let Some(histogram) = self.histograms.get(name) {
            return Some(MetricValue::Histogram(histogram.summary()));
        }
        None
    }
    
    /// Get all metrics
    pub fn all_metrics(&self) -> HashMap<String, MetricValue> {
        let mut result = HashMap::new();
        
        for (name, &value) in &self.gauges {
            result.insert(name.clone(), MetricValue::Gauge(value));
        }
        
        for (name, &value) in &self.counters {
            result.insert(name.clone(), MetricValue::Counter(value));
        }
        
        for (name, histogram) in &self.histograms {
            result.insert(name.clone(), MetricValue::Histogram(histogram.summary()));
        }
        
        result
    }
    
    /// Reset all metrics
    pub fn reset(&mut self) {
        self.gauges.clear();
        self.counters.clear();
        self.histograms.clear();
        self.frame_times.clear();
    }
    
    /// Get average FPS
    pub fn average_fps(&self) -> f64 {
        if self.frame_times.is_empty() {
            return 0.0;
        }
        
        let total: Duration = self.frame_times.iter().sum();
        let avg_frame_time = total.as_secs_f64() / self.frame_times.len() as f64;
        
        if avg_frame_time > 0.0 {
            1.0 / avg_frame_time
        } else {
            0.0
        }
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Metric value types
#[derive(Debug, Clone)]
pub enum MetricValue {
    Gauge(f64),
    Counter(u64),
    Histogram(HistogramSummary),
}

/// Histogram for distribution tracking
pub struct Histogram {
    /// Values
    values: Vec<f64>,
    /// Maximum values to keep
    max_values: usize,
    /// Sum of all values
    sum: f64,
    /// Count of all values
    count: u64,
    /// Minimum value
    min: f64,
    /// Maximum value
    max: f64,
}

impl Histogram {
    /// Create a new histogram
    pub fn new() -> Self {
        Self {
            values: Vec::new(),
            max_values: 10000,
            sum: 0.0,
            count: 0,
            min: f64::INFINITY,
            max: f64::NEG_INFINITY,
        }
    }
    
    /// Record a value
    pub fn record(&mut self, value: f64) {
        self.values.push(value);
        if self.values.len() > self.max_values {
            self.values.remove(0);
        }
        
        self.sum += value;
        self.count += 1;
        self.min = self.min.min(value);
        self.max = self.max.max(value);
    }
    
    /// Get summary statistics
    pub fn summary(&self) -> HistogramSummary {
        if self.values.is_empty() {
            return HistogramSummary::default();
        }
        
        let mut sorted = self.values.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let len = sorted.len();
        
        HistogramSummary {
            count: self.count,
            sum: self.sum,
            min: self.min,
            max: self.max,
            avg: self.sum / self.count as f64,
            p50: sorted[len / 2],
            p90: sorted[(len as f64 * 0.90) as usize],
            p95: sorted[(len as f64 * 0.95) as usize],
            p99: sorted[((len as f64 * 0.99) as usize).min(len - 1)],
        }
    }
}

impl Default for Histogram {
    fn default() -> Self {
        Self::new()
    }
}

/// Histogram summary
#[derive(Debug, Clone, Default)]
pub struct HistogramSummary {
    pub count: u64,
    pub sum: f64,
    pub min: f64,
    pub max: f64,
    pub avg: f64,
    pub p50: f64,
    pub p90: f64,
    pub p95: f64,
    pub p99: f64,
}

/// Predefined metric names
pub mod metric_names {
    pub const FRAME_TIME_MS: &str = "frame_time_ms";
    pub const FPS: &str = "fps";
    pub const DRAW_CALLS: &str = "draw_calls";
    pub const TRIANGLES: &str = "triangles";
    pub const VERTICES: &str = "vertices";
    pub const ENTITIES: &str = "entities";
    pub const CHUNKS_LOADED: &str = "chunks_loaded";
    pub const CHUNKS_RENDERED: &str = "chunks_rendered";
    pub const PARTICLES: &str = "particles";
    pub const MEMORY_USED: &str = "memory_used";
    pub const VRAM_USED: &str = "vram_used";
    pub const CPU_USAGE: &str = "cpu_usage";
    pub const GPU_USAGE: &str = "gpu_usage";
    pub const NETWORK_BYTES_IN: &str = "network_bytes_in";
    pub const NETWORK_BYTES_OUT: &str = "network_bytes_out";
    pub const AUDIO_SOURCES: &str = "audio_sources";
}
