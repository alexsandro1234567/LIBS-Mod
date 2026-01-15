//! # LIBS Core - Rust Native Engine
//! 
//! High-performance game engine for LIBS (Minecraft Engine Replacement).
//! 
//! ## Modules
//! 
//! - **Quantum Renderer**: Vulkan-based hybrid rendering with Nanite LOD
//! - **Hyper-Threaded ECS**: Parallel entity processing 
//! - **Void Manager**: Off-heap memory management (GC-free)
//! - **Predictive Netcode**: Delta compression & latency masking
//! - **Ray-Traced Audio**: Voxel-based occlusion & reverb
//! 
//! ## Author
//! 
//! Aiblox (Alexsandro Alves de Oliveira)

#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]

// Core modules
pub mod jni;
pub mod engine;
pub mod renderer;
pub mod memory;
pub mod ecs;
pub mod audio;
pub mod network;
pub mod world;
pub mod util;
pub mod profiling;
pub mod compat;

// Re-exports
pub use engine::AetherEngine;
pub use memory::MemoryManager;
pub use profiling::{profiler, Profiler};

// New module re-exports
pub use renderer::quantum::QuantumRenderer;
pub use ecs::EcsWorld;
pub use memory::void_manager::VoidManager;
pub use network::prediction::PredictiveNetcode;
pub use audio::raytracer::AudioRaytracer;
pub use compat::TheWeaver;
pub use renderer::bindless::BindlessTextureManager;
pub use renderer::quantum::greedy_mesh::GpuGreedyMesher;
pub use renderer::vulkan::interop::VulkanGLInterop;
pub use ecs::parallel::ParallelScheduler;
pub use world::NbtAssetLoader;
pub use renderer::quantum::nanite::NaniteManager;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Once;
use std::sync::Arc;
use parking_lot::RwLock;

/// Library version
pub const VERSION: &str = "1.0.0";

/// Library name  
pub const NAME: &str = "libs_core";

/// Initialization guard
static INIT: Once = Once::new();
static INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Global engine instance
static mut ENGINE: Option<Arc<RwLock<LibsEngine>>> = None;

/// LIBS Engine - Main entry point
pub struct LibsEngine {
    /// Quantum Renderer
    pub renderer: renderer::quantum::QuantumRenderer,
    /// ECS World
    pub ecs: ecs::EcsWorld,
    /// Void Manager (memory)
    pub memory: Arc<memory::void_manager::VoidManager>,
    /// Predictive Netcode
    pub netcode: network::prediction::PredictiveNetcode,
    /// Audio Raytracer
    pub audio: audio::raytracer::AudioRaytracer,
    /// Initialized flag
    initialized: bool,
}

impl LibsEngine {
    /// Create new engine
    pub fn new() -> Self {
        log::info!("Creating LIBS Engine instance...");
        
        Self {
            renderer: renderer::quantum::QuantumRenderer::new(),
            ecs: ecs::EcsWorld::new(),
            memory: memory::void_manager::VoidManager::new(),
            netcode: network::prediction::PredictiveNetcode::new(),
            audio: audio::raytracer::AudioRaytracer::new(),
            initialized: false,
        }
    }
    
    /// Initialize all subsystems
    pub fn initialize(&mut self) -> Result<(), String> {
        if self.initialized {
            return Ok(());
        }
        
        log::info!("╔════════════════════════════════════════════════════════════════╗");
        log::info!("║               LIBS ENGINE - Initializing                       ║");
        log::info!("╚════════════════════════════════════════════════════════════════╝");
        
        // Note: Vulkan init is deferred until window surface is available
        log::info!("Renderer: Ready (Vulkan init deferred)");
        log::info!("ECS: {} threads available", num_cpus::get() - 2);
        log::info!("Memory: Void Manager active");
        log::info!("Network: Predictive netcode enabled");
        log::info!("Audio: Ray-traced audio ready");
        
        self.initialized = true;
        
        log::info!("LIBS Engine initialized successfully!");
        
        Ok(())
    }
    
    /// Process one tick
    pub fn tick(&mut self, delta_time: f32) {
        if !self.initialized {
            return;
        }
        
        // ECS parallel tick
        self.ecs.parallel_tick(delta_time);
        
        // Process audio
        // (audio processing happens on separate thread)
    }
    
    /// Begin frame rendering
    pub fn begin_frame(&mut self) {
        if !self.initialized {
            return;
        }
        
        // Begin Vulkan frame (if initialized)
        if self.renderer.is_initialized() {
            let _ = self.renderer.begin_frame();
        }
    }
    
    /// End frame rendering
    pub fn end_frame(&mut self) {
        if !self.initialized {
            return;
        }
        
        self.renderer.end_frame();
    }
    
    /// Shutdown
    pub fn shutdown(&mut self) {
        log::info!("LIBS Engine shutting down...");
        
        self.renderer.shutdown();
        self.ecs.clear();
        self.memory.clear();
        self.netcode.clear();
        self.audio.clear();
        
        self.initialized = false;
        
        log::info!("LIBS Engine shutdown complete");
    }
    
    /// Check if initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
}

/// Initialize the LIBS library
pub fn initialize() -> bool {
    let mut success = false;
    
    INIT.call_once(|| {
        init_logging();
        
        log::info!("╔════════════════════════════════════════════════════════════════╗");
        log::info!("║                   LIBS CORE v{}                            ║", VERSION);
        log::info!("║           Native Engine by Aiblox                              ║");
        log::info!("╚════════════════════════════════════════════════════════════════╝");
        log::info!("Platform: {} {}", std::env::consts::OS, std::env::consts::ARCH);
        log::info!("CPU Cores: {}", num_cpus::get());
        
        // Create engine
        let mut engine = LibsEngine::new();
        
        if let Err(e) = engine.initialize() {
            log::error!("Engine init failed: {}", e);
            return;
        }
        
        // Store global instance
        unsafe {
            ENGINE = Some(Arc::new(RwLock::new(engine)));
        }
        
        INITIALIZED.store(true, Ordering::SeqCst);
        success = true;
        
        log::info!("LIBS Core initialization complete");
    });
    
    INITIALIZED.load(Ordering::SeqCst)
}

/// Get engine instance
pub fn get_engine() -> Option<Arc<RwLock<LibsEngine>>> {
    unsafe { ENGINE.clone() }
}

/// Check if initialized
pub fn is_initialized() -> bool {
    INITIALIZED.load(Ordering::SeqCst)
}

/// Shutdown
pub fn shutdown() {
    if let Some(engine) = get_engine() {
        engine.write().shutdown();
    }
    INITIALIZED.store(false, Ordering::SeqCst);
}

/// Initialize logging
fn init_logging() {
    use tracing_subscriber::fmt;
    use tracing_subscriber::prelude::*;
    
    let _ = tracing_subscriber::registry()
        .with(fmt::layer())
        .try_init();
}
