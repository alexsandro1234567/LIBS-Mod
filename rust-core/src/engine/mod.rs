//! # Aether Engine Core
//! 
//! The main engine orchestrator that coordinates all subsystems.

pub mod config;
pub mod state;

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Instant;

use crate::ecs::EcsWorld;
use crate::renderer::Renderer;
use crate::audio::AudioEngine;
use crate::world::WorldManager;

pub use config::EngineConfig;
pub use state::EngineState;

/// The main Aether Engine
pub struct AetherEngine {
    /// Engine configuration
    config: EngineConfig,
    
    /// Engine state
    state: EngineState,
    
    /// Running flag
    running: AtomicBool,
    
    /// Frame counter
    frame_count: AtomicU64,
    
    /// Last frame time
    last_frame_time: Instant,
    
    /// Frame delta time
    delta_time: f32,
    
    /// FPS tracking
    fps: f32,
    frame_times: Vec<f32>,
    
    /// ECS world
    ecs: Option<EcsWorld>,
    
    /// Renderer
    renderer: Option<Renderer>,
    
    /// Audio engine
    audio: Option<AudioEngine>,
    
    /// World manager
    world: Option<WorldManager>,
    
    /// Camera state
    camera: CameraState,
    
    /// Texture handles
    textures: HashMap<u64, TextureInfo>,
    next_texture_handle: AtomicU64,
    
    /// Debug flags
    debug_flags: HashMap<String, bool>,
    
    /// Profile data pointer (for external profilers)
    profile_data: Vec<u8>,
    
    /// Prediction state buffer
    prediction_buffer: Vec<PredictionState>,
}

/// Camera state
#[derive(Debug, Clone, Default)]
pub struct CameraState {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yaw: f32,
    pub pitch: f32,
}

/// Texture information
#[derive(Debug, Clone)]
pub struct TextureInfo {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub format: u32,
    pub data_ptr: *mut u8,
    pub data_len: usize,
}

/// Prediction state for netcode
#[derive(Debug, Clone)]
pub struct PredictionState {
    pub tick: u64,
    pub data: Vec<u8>,
}

impl AetherEngine {
    /// Create a new engine instance
    pub fn new(config_data: &[u8]) -> Result<Self, String> {
        log::info!("Creating AetherEngine...");
        
        // Parse configuration
        let config = EngineConfig::from_bytes(config_data)?;
        
        log::info!("Engine config loaded:");
        log::info!("  Render mode: {:?}", config.render_mode);
        log::info!("  Max off-heap memory: {} MB", config.max_offheap_mb);
        
        // Initialize subsystems
        let ecs = Some(EcsWorld::new());
        log::info!("  ECS initialized");
        
        let renderer = match Renderer::new(&config) {
            Ok(r) => {
                log::info!("  Renderer initialized");
                Some(r)
            }
            Err(e) => {
                log::warn!("  Renderer init failed: {} - running headless", e);
                None
            }
        };
        
        let audio = match AudioEngine::new() {
            Ok(a) => {
                log::info!("  Audio engine initialized");
                Some(a)
            }
            Err(e) => {
                log::warn!("  Audio init failed: {}", e);
                None
            }
        };
        
        let world = Some(WorldManager::new());
        log::info!("  World manager initialized");
        
        Ok(Self {
            config,
            state: EngineState::new(),
            running: AtomicBool::new(true),
            frame_count: AtomicU64::new(0),
            last_frame_time: Instant::now(),
            delta_time: 0.016,
            fps: 60.0,
            frame_times: Vec::with_capacity(60),
            ecs,
            renderer,
            audio,
            world,
            camera: CameraState::default(),
            textures: HashMap::new(),
            next_texture_handle: AtomicU64::new(1),
            debug_flags: HashMap::new(),
            profile_data: Vec::new(),
            prediction_buffer: Vec::new(),
        })
    }
    
    /// Check if engine is running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
    
    /// Shutdown the engine
    pub fn shutdown(&self) {
        log::info!("Engine shutting down...");
        self.running.store(false, Ordering::SeqCst);
    }
    
    /// Process a game tick
    pub fn tick(&mut self, delta_time: f32) {
        self.delta_time = delta_time;
        
        // Update ECS
        if let Some(ref mut ecs) = self.ecs {
            ecs.tick(delta_time);
        }
        
        // Update world
        if let Some(ref mut world) = self.world {
            world.tick();
        }
        
        // Update audio listener
        if let Some(ref mut audio) = self.audio {
            audio.update_listener(
                self.camera.x as f32,
                self.camera.y as f32,
                self.camera.z as f32,
                self.camera.yaw,
                self.camera.pitch,
            );
        }
    }
    
    /// Begin rendering a frame
    pub fn begin_frame(&mut self, _partial_ticks: f32) {
        // Calculate frame time
        let now = Instant::now();
        let frame_time = now.duration_since(self.last_frame_time).as_secs_f32();
        self.last_frame_time = now;
        
        // Track FPS
        self.frame_times.push(frame_time);
        if self.frame_times.len() > 60 {
            self.frame_times.remove(0);
        }
        let avg_frame_time: f32 = self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32;
        self.fps = 1.0 / avg_frame_time;
        
        // Begin renderer frame
        if let Some(ref mut renderer) = self.renderer {
            renderer.begin_frame();
        }
    }
    
    /// End rendering a frame
    pub fn end_frame(&mut self) {
        // End renderer frame
        if let Some(ref mut renderer) = self.renderer {
            renderer.end_frame();
        }
        
        self.frame_count.fetch_add(1, Ordering::SeqCst);
    }
    
    /// Update camera state
    pub fn update_camera(&mut self, x: f64, y: f64, z: f64, yaw: f32, pitch: f32) {
        self.camera.x = x;
        self.camera.y = y;
        self.camera.z = z;
        self.camera.yaw = yaw;
        self.camera.pitch = pitch;
        
        // Update renderer camera
        if let Some(ref mut renderer) = self.renderer {
            renderer.set_camera(x, y, z, yaw, pitch);
        }
    }
    
    // ========================================================================
    // CHUNK FUNCTIONS
    // ========================================================================
    
    /// Submit chunk data
    pub fn submit_chunk(&mut self, x: i32, z: i32, data: &[u8]) -> i64 {
        if let Some(ref mut world) = self.world {
            world.submit_chunk(x, z, data)
        } else {
            0
        }
    }
    
    /// Update chunk data
    pub fn update_chunk(&mut self, x: i32, z: i32, data: &[u8]) {
        if let Some(ref mut world) = self.world {
            world.update_chunk(x, z, data);
        }
    }
    
    /// Mark chunk as dirty (needs re-mesh)
    pub fn mark_chunk_dirty(&mut self, x: i32, z: i32) {
        if let Some(ref mut world) = self.world {
            world.mark_chunk_dirty(x, z);
        }
    }
    
    /// Unload a chunk
    pub fn unload_chunk(&mut self, x: i32, z: i32) {
        if let Some(ref mut world) = self.world {
            world.unload_chunk(x, z);
        }
    }
    
    /// Set a block
    pub fn set_block(&mut self, x: i32, y: i32, z: i32, block_id: u32) {
        if let Some(ref mut world) = self.world {
            world.set_block(x, y, z, block_id);
        }
    }
    
    // ========================================================================
    // ENTITY FUNCTIONS
    // ========================================================================
    
    /// Register an entity
    pub fn register_entity(&mut self, entity_id: i32, entity_type: i32, x: f64, y: f64, z: f64) -> i64 {
        if let Some(ref mut ecs) = self.ecs {
            ecs.spawn_entity(entity_id, entity_type, x, y, z)
        } else {
            0
        }
    }
    
    /// Update entity position
    pub fn update_entity(&mut self, handle: u64, x: f64, y: f64, z: f64, yaw: f32, pitch: f32) {
        if let Some(ref mut ecs) = self.ecs {
            ecs.update_entity(handle, x, y, z, yaw, pitch);
        }
    }
    
    /// Remove an entity
    pub fn remove_entity(&mut self, handle: u64) {
        if let Some(ref mut ecs) = self.ecs {
            ecs.despawn_entity(handle);
        }
    }
    
    // ========================================================================
    // TEXTURE FUNCTIONS
    // ========================================================================
    
    /// Upload a texture
    pub fn upload_texture(&mut self, name: &str, data: &[u8], width: u32, height: u32, format: u32) -> i64 {
        let handle = self.next_texture_handle.fetch_add(1, Ordering::SeqCst);
        
        // Allocate and copy texture data
        let data_copy = data.to_vec();
        let data_ptr = data_copy.as_ptr() as *mut u8;
        let data_len = data_copy.len();
        std::mem::forget(data_copy); // Prevent deallocation
        
        let info = TextureInfo {
            name: name.to_string(),
            width,
            height,
            format,
            data_ptr,
            data_len,
        };
        
        self.textures.insert(handle, info);
        
        // Upload to renderer if available
        if let Some(ref mut renderer) = self.renderer {
            renderer.upload_texture(handle, name, data, width, height, format);
        }
        
        log::trace!("Texture uploaded: {} ({}x{}) -> handle {}", name, width, height, handle);
        
        handle as i64
    }
    
    /// Unload a texture
    pub fn unload_texture(&mut self, handle: u64) {
        if let Some(info) = self.textures.remove(&handle) {
            // Free the texture data
            unsafe {
                let _ = Vec::from_raw_parts(info.data_ptr, info.data_len, info.data_len);
            }
            
            // Unload from renderer
            if let Some(ref mut renderer) = self.renderer {
                renderer.unload_texture(handle);
            }
            
            log::trace!("Texture unloaded: handle {}", handle);
        }
    }
    
    // ========================================================================
    // AUDIO FUNCTIONS
    // ========================================================================
    
    /// Play a sound
    pub fn play_sound(&mut self, name: &str, x: f32, y: f32, z: f32, volume: f32, pitch: f32) -> i64 {
        if let Some(ref mut audio) = self.audio {
            audio.play(name, x, y, z, volume, pitch) as i64
        } else {
            0
        }
    }
    
    /// Stop a sound by handle
    pub fn stop_sound(&mut self, handle: u64) {
        if let Some(ref mut audio) = self.audio {
            audio.stop(handle);
        }
    }
    
    /// Stop a sound by name
    pub fn stop_sound_by_name(&mut self, name: &str) {
        if let Some(ref mut audio) = self.audio {
            audio.stop_by_name(name);
        }
    }
    
    /// Update listener position
    pub fn update_listener(&mut self, x: f32, y: f32, z: f32, yaw: f32, pitch: f32) {
        if let Some(ref mut audio) = self.audio {
            audio.update_listener(x, y, z, yaw, pitch);
        }
    }
    
    // ========================================================================
    // NETWORK/PREDICTION FUNCTIONS
    // ========================================================================
    
    /// Store prediction state for a tick
    pub fn predict_state(&mut self, tick: u64, state: &[u8]) {
        // Remove old predictions
        self.prediction_buffer.retain(|p| tick.saturating_sub(p.tick) < 128);
        
        self.prediction_buffer.push(PredictionState {
            tick,
            data: state.to_vec(),
        });
    }
    
    /// Reconcile with server state
    pub fn reconcile_state(&mut self, tick: u64, server_state: &[u8]) {
        // Find the prediction for this tick
        if let Some(prediction) = self.prediction_buffer.iter().find(|p| p.tick == tick) {
            // Compare prediction with server state
            if prediction.data != server_state {
                log::debug!("State mismatch at tick {}, reconciling...", tick);
                // In a full implementation, would replay inputs from this tick
            }
        }
        
        // Remove predictions up to this tick
        self.prediction_buffer.retain(|p| p.tick > tick);
    }
    
    // ========================================================================
    // DEBUG FUNCTIONS
    // ========================================================================
    
    /// Get debug information string
    pub fn get_debug_info(&self) -> String {
        let ecs_count = self.ecs.as_ref().map(|e| e.entity_count()).unwrap_or(0);
        let chunk_count = self.world.as_ref().map(|w| w.chunk_count()).unwrap_or(0);
        let texture_count = self.textures.len();
        let sound_count = self.audio.as_ref().map(|a| a.sound_count()).unwrap_or(0);
        
        format!(
            "Aether Engine v{}\n\
             FPS: {:.1}\n\
             Frame: {}\n\
             Entities: {}\n\
             Chunks: {}\n\
             Textures: {}\n\
             Sounds: {}\n\
             Memory: {} bytes\n\
             Render Mode: {:?}",
            crate::VERSION,
            self.fps,
            self.frame_count.load(Ordering::SeqCst),
            ecs_count,
            chunk_count,
            texture_count,
            sound_count,
            crate::memory::MemoryManager::get_allocated_bytes(),
            self.config.render_mode
        )
    }
    
    /// Set a debug flag
    pub fn set_debug_flag(&mut self, flag: &str, value: bool) {
        self.debug_flags.insert(flag.to_string(), value);
        log::debug!("Debug flag '{}' set to {}", flag, value);
    }
    
    /// Get a debug flag
    pub fn get_debug_flag(&self, flag: &str) -> bool {
        self.debug_flags.get(flag).copied().unwrap_or(false)
    }
    
    /// Get profile data pointer
    pub fn get_profile_data_ptr(&self) -> i64 {
        self.profile_data.as_ptr() as i64
    }
    
    /// Get FPS
    pub fn get_fps(&self) -> f32 {
        self.fps
    }
    
    /// Get frame time in milliseconds
    pub fn get_frame_time_ms(&self) -> f32 {
        if self.fps > 0.0 {
            1000.0 / self.fps
        } else {
            0.0
        }
    }
    
    /// Get frame count
    pub fn get_frame_count(&self) -> u64 {
        self.frame_count.load(Ordering::SeqCst)
    }
}

// Ensure TextureInfo is Send + Sync (raw pointer needs explicit impl)
unsafe impl Send for TextureInfo {}
unsafe impl Sync for TextureInfo {}
