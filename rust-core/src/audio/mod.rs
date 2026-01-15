//! # Audio Engine Module
//! 
//! 3D positional audio system.

pub mod raytracer;

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

/// Next sound handle
static NEXT_SOUND_HANDLE: AtomicU64 = AtomicU64::new(1);

/// Audio engine
pub struct AudioEngine {
    /// Master volume
    master_volume: f32,
    
    /// Active sounds
    sounds: HashMap<u64, SoundInstance>,
    
    /// Sound name to handle mapping
    name_to_handles: HashMap<String, Vec<u64>>,
    
    /// Listener position
    listener_x: f32,
    listener_y: f32,
    listener_z: f32,
    listener_yaw: f32,
    listener_pitch: f32,
}

/// A playing sound instance
struct SoundInstance {
    name: String,
    x: f32,
    y: f32,
    z: f32,
    volume: f32,
    pitch: f32,
    playing: bool,
}

impl AudioEngine {
    /// Create a new audio engine
    pub fn new() -> Result<Self, String> {
        log::debug!("Audio engine created");
        
        Ok(Self {
            master_volume: 1.0,
            sounds: HashMap::new(),
            name_to_handles: HashMap::new(),
            listener_x: 0.0,
            listener_y: 0.0,
            listener_z: 0.0,
            listener_yaw: 0.0,
            listener_pitch: 0.0,
        })
    }
    
    /// Play a sound
    pub fn play(&mut self, name: &str, x: f32, y: f32, z: f32, volume: f32, pitch: f32) -> u64 {
        let handle = NEXT_SOUND_HANDLE.fetch_add(1, Ordering::SeqCst);
        
        let instance = SoundInstance {
            name: name.to_string(),
            x,
            y,
            z,
            volume,
            pitch,
            playing: true,
        };
        
        self.sounds.insert(handle, instance);
        
        // Track by name
        self.name_to_handles
            .entry(name.to_string())
            .or_insert_with(Vec::new)
            .push(handle);
        
        log::trace!("Sound playing: {} at ({}, {}, {})", name, x, y, z);
        
        handle
    }
    
    /// Stop a sound by handle
    pub fn stop(&mut self, handle: u64) {
        if let Some(sound) = self.sounds.remove(&handle) {
            // Remove from name mapping
            if let Some(handles) = self.name_to_handles.get_mut(&sound.name) {
                handles.retain(|&h| h != handle);
            }
            log::trace!("Sound stopped: handle {}", handle);
        }
    }
    
    /// Stop all sounds with a given name
    pub fn stop_by_name(&mut self, name: &str) {
        if let Some(handles) = self.name_to_handles.remove(name) {
            for handle in handles {
                self.sounds.remove(&handle);
            }
            log::trace!("Stopped all sounds named: {}", name);
        }
    }
    
    /// Stop all sounds
    pub fn stop_all(&mut self) {
        self.sounds.clear();
        self.name_to_handles.clear();
        log::trace!("All sounds stopped");
    }
    
    /// Update listener position
    pub fn update_listener(&mut self, x: f32, y: f32, z: f32, yaw: f32, pitch: f32) {
        self.listener_x = x;
        self.listener_y = y;
        self.listener_z = z;
        self.listener_yaw = yaw;
        self.listener_pitch = pitch;
    }
    
    /// Set master volume
    pub fn set_master_volume(&mut self, volume: f32) {
        self.master_volume = volume.clamp(0.0, 1.0);
    }
    
    /// Get master volume
    pub fn get_master_volume(&self) -> f32 {
        self.master_volume
    }
    
    /// Get sound count
    pub fn sound_count(&self) -> usize {
        self.sounds.len()
    }
    
    /// Calculate 3D audio attenuation
    pub fn calculate_attenuation(&self, x: f32, y: f32, z: f32) -> f32 {
        let dx = x - self.listener_x;
        let dy = y - self.listener_y;
        let dz = z - self.listener_z;
        let distance = (dx * dx + dy * dy + dz * dz).sqrt();
        
        // Simple linear attenuation with max distance of 16 blocks
        let max_distance = 16.0;
        let attenuation = 1.0 - (distance / max_distance).min(1.0);
        
        attenuation * self.master_volume
    }
}

/// Shutdown audio subsystem
pub fn shutdown() {
    log::debug!("Audio subsystem shutdown");
}
