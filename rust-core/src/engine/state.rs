//! # Engine State
//! 
//! Runtime state tracking for the engine.

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::Instant;

/// Engine initialization state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum InitState {
    Uninitialized = 0,
    Initializing = 1,
    Ready = 2,
    Running = 3,
    Paused = 4,
    Stopping = 5,
    Stopped = 6,
    Error = 255,
}

impl From<u32> for InitState {
    fn from(v: u32) -> Self {
        match v {
            0 => InitState::Uninitialized,
            1 => InitState::Initializing,
            2 => InitState::Ready,
            3 => InitState::Running,
            4 => InitState::Paused,
            5 => InitState::Stopping,
            6 => InitState::Stopped,
            _ => InitState::Error,
        }
    }
}

/// Engine state container
pub struct EngineState {
    /// Current init state
    state: AtomicU32,
    
    /// Start time
    start_time: Instant,
    
    /// Total ticks processed
    tick_count: AtomicU64,
    
    /// Total frames rendered
    frame_count: AtomicU64,
    
    /// Entities spawned
    entity_count: AtomicU64,
    
    /// Chunks loaded
    chunk_count: AtomicU64,
}

impl EngineState {
    /// Create new engine state
    pub fn new() -> Self {
        Self {
            state: AtomicU32::new(InitState::Uninitialized as u32),
            start_time: Instant::now(),
            tick_count: AtomicU64::new(0),
            frame_count: AtomicU64::new(0),
            entity_count: AtomicU64::new(0),
            chunk_count: AtomicU64::new(0),
        }
    }
    
    /// Get current state
    pub fn get_state(&self) -> InitState {
        InitState::from(self.state.load(Ordering::SeqCst))
    }
    
    /// Set state
    pub fn set_state(&self, state: InitState) {
        self.state.store(state as u32, Ordering::SeqCst);
    }
    
    /// Increment tick count
    pub fn tick(&self) {
        self.tick_count.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Increment frame count
    pub fn frame(&self) {
        self.frame_count.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Get uptime in seconds
    pub fn uptime_secs(&self) -> f64 {
        self.start_time.elapsed().as_secs_f64()
    }
    
    /// Get tick count
    pub fn get_tick_count(&self) -> u64 {
        self.tick_count.load(Ordering::Relaxed)
    }
    
    /// Get frame count
    pub fn get_frame_count(&self) -> u64 {
        self.frame_count.load(Ordering::Relaxed)
    }
    
    /// Get entity count
    pub fn get_entity_count(&self) -> u64 {
        self.entity_count.load(Ordering::Relaxed)
    }
    
    /// Increment entity count
    pub fn entity_spawned(&self) {
        self.entity_count.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Decrement entity count
    pub fn entity_despawned(&self) {
        self.entity_count.fetch_sub(1, Ordering::Relaxed);
    }
    
    /// Get chunk count
    pub fn get_chunk_count(&self) -> u64 {
        self.chunk_count.load(Ordering::Relaxed)
    }
    
    /// Increment chunk count
    pub fn chunk_loaded(&self) {
        self.chunk_count.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Decrement chunk count
    pub fn chunk_unloaded(&self) {
        self.chunk_count.fetch_sub(1, Ordering::Relaxed);
    }
}

impl Default for EngineState {
    fn default() -> Self {
        Self::new()
    }
}
