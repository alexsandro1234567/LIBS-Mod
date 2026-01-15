//! # High-Resolution Timer
//! 
//! High-resolution timing utilities for profiling.

use std::time::{Duration, Instant};

/// High-resolution timer
pub struct Timer {
    start: Instant,
    accumulated: Duration,
    running: bool,
}

impl Timer {
    /// Create a new timer (not started)
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
            accumulated: Duration::ZERO,
            running: false,
        }
    }
    
    /// Create and start a new timer
    pub fn start_new() -> Self {
        let mut timer = Self::new();
        timer.start();
        timer
    }
    
    /// Start the timer
    pub fn start(&mut self) {
        if !self.running {
            self.start = Instant::now();
            self.running = true;
        }
    }
    
    /// Stop the timer
    pub fn stop(&mut self) {
        if self.running {
            self.accumulated += self.start.elapsed();
            self.running = false;
        }
    }
    
    /// Reset the timer
    pub fn reset(&mut self) {
        self.accumulated = Duration::ZERO;
        self.running = false;
    }
    
    /// Restart the timer
    pub fn restart(&mut self) {
        self.reset();
        self.start();
    }
    
    /// Get elapsed time
    pub fn elapsed(&self) -> Duration {
        if self.running {
            self.accumulated + self.start.elapsed()
        } else {
            self.accumulated
        }
    }
    
    /// Get elapsed time in milliseconds
    pub fn elapsed_ms(&self) -> f64 {
        self.elapsed().as_secs_f64() * 1000.0
    }
    
    /// Get elapsed time in microseconds
    pub fn elapsed_us(&self) -> f64 {
        self.elapsed().as_secs_f64() * 1_000_000.0
    }
    
    /// Get elapsed time in nanoseconds
    pub fn elapsed_ns(&self) -> u128 {
        self.elapsed().as_nanos()
    }
    
    /// Check if timer is running
    pub fn is_running(&self) -> bool {
        self.running
    }
}

impl Default for Timer {
    fn default() -> Self {
        Self::new()
    }
}

/// Stopwatch for measuring multiple laps
pub struct Stopwatch {
    timer: Timer,
    laps: Vec<Duration>,
}

impl Stopwatch {
    /// Create a new stopwatch
    pub fn new() -> Self {
        Self {
            timer: Timer::new(),
            laps: Vec::new(),
        }
    }
    
    /// Start the stopwatch
    pub fn start(&mut self) {
        self.timer.start();
    }
    
    /// Record a lap
    pub fn lap(&mut self) -> Duration {
        let elapsed = self.timer.elapsed();
        let lap_time = if let Some(last) = self.laps.last() {
            elapsed - *last
        } else {
            elapsed
        };
        self.laps.push(elapsed);
        lap_time
    }
    
    /// Stop the stopwatch
    pub fn stop(&mut self) {
        self.timer.stop();
    }
    
    /// Reset the stopwatch
    pub fn reset(&mut self) {
        self.timer.reset();
        self.laps.clear();
    }
    
    /// Get total elapsed time
    pub fn elapsed(&self) -> Duration {
        self.timer.elapsed()
    }
    
    /// Get all lap times
    pub fn laps(&self) -> &[Duration] {
        &self.laps
    }
    
    /// Get lap count
    pub fn lap_count(&self) -> usize {
        self.laps.len()
    }
    
    /// Get average lap time
    pub fn average_lap(&self) -> Duration {
        if self.laps.is_empty() {
            return Duration::ZERO;
        }
        
        let total: Duration = self.laps.iter().sum();
        total / self.laps.len() as u32
    }
}

impl Default for Stopwatch {
    fn default() -> Self {
        Self::new()
    }
}

/// Frame time tracker
pub struct FrameTimer {
    /// Last frame time
    last_frame_time: Duration,
    /// Frame time history
    history: Vec<Duration>,
    /// Maximum history size
    max_history: usize,
    /// Frame start time
    frame_start: Instant,
    /// Total frames
    total_frames: u64,
    /// Total time
    total_time: Duration,
}

impl FrameTimer {
    /// Create a new frame timer
    pub fn new(max_history: usize) -> Self {
        Self {
            last_frame_time: Duration::ZERO,
            history: Vec::with_capacity(max_history),
            max_history,
            frame_start: Instant::now(),
            total_frames: 0,
            total_time: Duration::ZERO,
        }
    }
    
    /// Begin a new frame
    pub fn begin_frame(&mut self) {
        self.frame_start = Instant::now();
    }
    
    /// End the current frame
    pub fn end_frame(&mut self) {
        self.last_frame_time = self.frame_start.elapsed();
        self.total_time += self.last_frame_time;
        self.total_frames += 1;
        
        self.history.push(self.last_frame_time);
        if self.history.len() > self.max_history {
            self.history.remove(0);
        }
    }
    
    /// Get last frame time
    pub fn last_frame_time(&self) -> Duration {
        self.last_frame_time
    }
    
    /// Get last frame time in milliseconds
    pub fn last_frame_time_ms(&self) -> f64 {
        self.last_frame_time.as_secs_f64() * 1000.0
    }
    
    /// Get average frame time
    pub fn average_frame_time(&self) -> Duration {
        if self.history.is_empty() {
            return Duration::ZERO;
        }
        
        let total: Duration = self.history.iter().sum();
        total / self.history.len() as u32
    }
    
    /// Get average FPS
    pub fn average_fps(&self) -> f64 {
        let avg = self.average_frame_time();
        if avg.as_secs_f64() > 0.0 {
            1.0 / avg.as_secs_f64()
        } else {
            0.0
        }
    }
    
    /// Get current FPS
    pub fn current_fps(&self) -> f64 {
        if self.last_frame_time.as_secs_f64() > 0.0 {
            1.0 / self.last_frame_time.as_secs_f64()
        } else {
            0.0
        }
    }
    
    /// Get frame time percentile
    pub fn percentile(&self, p: f64) -> Duration {
        if self.history.is_empty() {
            return Duration::ZERO;
        }
        
        let mut sorted = self.history.clone();
        sorted.sort();
        
        let index = ((sorted.len() as f64 * p) as usize).min(sorted.len() - 1);
        sorted[index]
    }
    
    /// Get total frames
    pub fn total_frames(&self) -> u64 {
        self.total_frames
    }
    
    /// Get total time
    pub fn total_time(&self) -> Duration {
        self.total_time
    }
    
    /// Reset the frame timer
    pub fn reset(&mut self) {
        self.last_frame_time = Duration::ZERO;
        self.history.clear();
        self.total_frames = 0;
        self.total_time = Duration::ZERO;
    }
}

impl Default for FrameTimer {
    fn default() -> Self {
        Self::new(300)
    }
}

/// Benchmark runner
pub struct Benchmark {
    name: String,
    iterations: u32,
    warmup_iterations: u32,
    results: Vec<Duration>,
}

impl Benchmark {
    /// Create a new benchmark
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            iterations: 1000,
            warmup_iterations: 100,
            results: Vec::new(),
        }
    }
    
    /// Set number of iterations
    pub fn iterations(mut self, n: u32) -> Self {
        self.iterations = n;
        self
    }
    
    /// Set warmup iterations
    pub fn warmup(mut self, n: u32) -> Self {
        self.warmup_iterations = n;
        self
    }
    
    /// Run the benchmark
    pub fn run<F: FnMut()>(&mut self, mut f: F) -> BenchmarkResult {
        // Warmup
        for _ in 0..self.warmup_iterations {
            f();
        }
        
        // Benchmark
        self.results.clear();
        for _ in 0..self.iterations {
            let start = Instant::now();
            f();
            self.results.push(start.elapsed());
        }
        
        self.calculate_result()
    }
    
    /// Calculate benchmark result
    fn calculate_result(&self) -> BenchmarkResult {
        if self.results.is_empty() {
            return BenchmarkResult::default();
        }
        
        let mut sorted: Vec<f64> = self.results.iter()
            .map(|d| d.as_secs_f64() * 1_000_000_000.0) // Convert to nanoseconds
            .collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let sum: f64 = sorted.iter().sum();
        let avg = sum / sorted.len() as f64;
        
        // Calculate standard deviation
        let variance: f64 = sorted.iter()
            .map(|x| (x - avg).powi(2))
            .sum::<f64>() / sorted.len() as f64;
        let std_dev = variance.sqrt();
        
        BenchmarkResult {
            name: self.name.clone(),
            iterations: self.iterations,
            min_ns: sorted[0],
            max_ns: sorted[sorted.len() - 1],
            avg_ns: avg,
            std_dev_ns: std_dev,
            p50_ns: sorted[sorted.len() / 2],
            p95_ns: sorted[(sorted.len() as f64 * 0.95) as usize],
            p99_ns: sorted[((sorted.len() as f64 * 0.99) as usize).min(sorted.len() - 1)],
        }
    }
}

/// Benchmark result
#[derive(Debug, Clone, Default)]
pub struct BenchmarkResult {
    pub name: String,
    pub iterations: u32,
    pub min_ns: f64,
    pub max_ns: f64,
    pub avg_ns: f64,
    pub std_dev_ns: f64,
    pub p50_ns: f64,
    pub p95_ns: f64,
    pub p99_ns: f64,
}

impl BenchmarkResult {
    /// Print result
    pub fn print(&self) {
        println!("Benchmark: {}", self.name);
        println!("  Iterations: {}", self.iterations);
        println!("  Min:    {:.2} ns", self.min_ns);
        println!("  Max:    {:.2} ns", self.max_ns);
        println!("  Avg:    {:.2} ns", self.avg_ns);
        println!("  StdDev: {:.2} ns", self.std_dev_ns);
        println!("  P50:    {:.2} ns", self.p50_ns);
        println!("  P95:    {:.2} ns", self.p95_ns);
        println!("  P99:    {:.2} ns", self.p99_ns);
    }
}
