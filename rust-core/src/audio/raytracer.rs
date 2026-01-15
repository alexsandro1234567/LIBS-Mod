//! Ray-Traced Audio Module
//!
//! Real-time audio processing:
//! - Voxel-based occlusion
//! - Geometry-based reverb
//! - Material-based absorption

use std::sync::Arc;
use glam::Vec3;
use std::collections::HashMap;

/// Audio source
pub struct AudioSource {
    pub id: u32,
    pub position: Vec3,
    pub volume: f32,
    pub pitch: f32,
    pub sound_id: u32,
    pub looping: bool,
    pub max_distance: f32,
}

/// Material acoustic properties
#[derive(Clone, Copy)]
pub struct MaterialAcoustics {
    /// Absorption coefficient (0-1)
    pub absorption: f32,
    /// Transmission coefficient (0-1)
    pub transmission: f32,
    /// Reflection coefficient (0-1)
    pub reflection: f32,
}

/// Pre-defined materials
impl MaterialAcoustics {
    pub fn stone() -> Self {
        Self { absorption: 0.02, transmission: 0.01, reflection: 0.97 }
    }
    
    pub fn wood() -> Self {
        Self { absorption: 0.15, transmission: 0.05, reflection: 0.80 }
    }
    
    pub fn wool() -> Self {
        Self { absorption: 0.70, transmission: 0.10, reflection: 0.20 }
    }
    
    pub fn glass() -> Self {
        Self { absorption: 0.03, transmission: 0.40, reflection: 0.57 }
    }
    
    pub fn water() -> Self {
        Self { absorption: 0.05, transmission: 0.80, reflection: 0.15 }
    }
    
    pub fn air() -> Self {
        Self { absorption: 0.0, transmission: 1.0, reflection: 0.0 }
    }
}

/// Reverb parameters
#[derive(Clone)]
pub struct ReverbParams {
    pub decay_time: f32,
    pub wet_level: f32,
    pub dry_level: f32,
    pub room_size: f32,
    pub diffusion: f32,
}

impl Default for ReverbParams {
    fn default() -> Self {
        Self {
            decay_time: 1.0,
            wet_level: 0.3,
            dry_level: 0.7,
            room_size: 0.5,
            diffusion: 0.5,
        }
    }
}

/// Ray-traced audio processor
pub struct AudioRaytracer {
    /// Listener position
    listener_pos: Vec3,
    /// Listener direction
    listener_dir: Vec3,
    /// Active audio sources
    sources: HashMap<u32, AudioSource>,
    /// Block material lookup
    materials: HashMap<u32, MaterialAcoustics>,
    /// Cached occlusion values
    occlusion_cache: HashMap<(u32, [i32; 3]), f32>,
    /// Current reverb params
    reverb: ReverbParams,
    /// Statistics
    stats: AudioStats,
}

/// Audio statistics
#[derive(Default, Clone)]
pub struct AudioStats {
    pub active_sources: u32,
    pub rays_traced: u32,
    pub cache_hits: u32,
    pub reverb_calculated: u32,
}

impl AudioRaytracer {
    /// Create new audio raytracer
    pub fn new() -> Self {
        log::info!("Initializing Ray-Traced Audio");
        
        // Register default materials
        let mut materials = HashMap::new();
        
        // Vanilla blocks
        materials.insert(1, MaterialAcoustics::stone());   // Stone
        materials.insert(4, MaterialAcoustics::stone());   // Cobblestone
        materials.insert(5, MaterialAcoustics::wood());    // Planks
        materials.insert(17, MaterialAcoustics::wood());   // Log
        materials.insert(20, MaterialAcoustics::glass());  // Glass
        materials.insert(35, MaterialAcoustics::wool());   // Wool
        materials.insert(9, MaterialAcoustics::water());   // Water
        
        Self {
            listener_pos: Vec3::ZERO,
            listener_dir: Vec3::Z,
            sources: HashMap::new(),
            materials,
            occlusion_cache: HashMap::new(),
            reverb: ReverbParams::default(),
            stats: AudioStats::default(),
        }
    }
    
    /// Update listener position
    pub fn update_listener(&mut self, position: Vec3, direction: Vec3) {
        // Clear cache if listener moved significantly
        if (self.listener_pos - position).length() > 1.0 {
            self.occlusion_cache.clear();
        }
        
        self.listener_pos = position;
        self.listener_dir = direction.normalize();
    }
    
    /// Add or update audio source
    pub fn set_source(&mut self, source: AudioSource) {
        self.sources.insert(source.id, source);
        self.stats.active_sources = self.sources.len() as u32;
    }
    
    /// Remove audio source
    pub fn remove_source(&mut self, id: u32) {
        self.sources.remove(&id);
        self.stats.active_sources = self.sources.len() as u32;
    }
    
    /// Calculate occlusion between source and listener
    pub fn calculate_occlusion(
        &mut self,
        source_id: u32,
        blocks: &impl Fn(i32, i32, i32) -> u32,
    ) -> f32 {
        let source = match self.sources.get(&source_id) {
            Some(s) => s,
            None => return 0.0,
        };
        
        let cache_key = (source_id, [
            source.position.x as i32,
            source.position.y as i32,
            source.position.z as i32,
        ]);
        
        // Check cache
        if let Some(&occlusion) = self.occlusion_cache.get(&cache_key) {
            self.stats.cache_hits += 1;
            return occlusion;
        }
        
        // Trace ray from source to listener
        let direction = (self.listener_pos - source.position).normalize();
        let distance = (self.listener_pos - source.position).length();
        
        let mut occlusion = 0.0;
        let step_size = 0.5;
        let mut current_dist = step_size;
        
        while current_dist < distance {
            let pos = source.position + direction * current_dist;
            let block_pos = [
                pos.x.floor() as i32,
                pos.y.floor() as i32,
                pos.z.floor() as i32,
            ];
            
            let block_id = blocks(block_pos[0], block_pos[1], block_pos[2]);
            
            if block_id != 0 {
                // Get material acoustics
                let material = self.materials
                    .get(&block_id)
                    .copied()
                    .unwrap_or(MaterialAcoustics::stone());
                
                // Accumulate occlusion
                occlusion += 1.0 - material.transmission;
            }
            
            current_dist += step_size;
            self.stats.rays_traced += 1;
        }
        
        // Clamp occlusion
        let occlusion = occlusion.min(1.0);
        
        // Cache result
        self.occlusion_cache.insert(cache_key, occlusion);
        
        occlusion
    }
    
    /// Calculate reverb parameters based on environment
    pub fn calculate_reverb(
        &mut self,
        blocks: &impl Fn(i32, i32, i32) -> u32,
    ) -> ReverbParams {
        let mut reverb = ReverbParams::default();
        
        // Cast rays in 6 directions to estimate room size
        let directions = [
            Vec3::X, Vec3::NEG_X,
            Vec3::Y, Vec3::NEG_Y,
            Vec3::Z, Vec3::NEG_Z,
        ];
        
        let mut total_distance = 0.0;
        let mut hits = 0;
        let mut absorption_sum = 0.0;
        
        for dir in directions {
            let max_dist = 32.0;
            let step = 0.5;
            let mut dist = step;
            
            while dist < max_dist {
                let pos = self.listener_pos + dir * dist;
                let block_pos = [
                    pos.x.floor() as i32,
                    pos.y.floor() as i32,
                    pos.z.floor() as i32,
                ];
                
                let block_id = blocks(block_pos[0], block_pos[1], block_pos[2]);
                
                if block_id != 0 {
                    total_distance += dist;
                    hits += 1;
                    
                    let material = self.materials
                        .get(&block_id)
                        .copied()
                        .unwrap_or(MaterialAcoustics::stone());
                    
                    absorption_sum += material.absorption;
                    break;
                }
                
                dist += step;
            }
        }
        
        if hits > 0 {
            let avg_distance = total_distance / hits as f32;
            let avg_absorption = absorption_sum / hits as f32;
            
            // Calculate reverb params
            reverb.room_size = (avg_distance / 32.0).clamp(0.0, 1.0);
            reverb.decay_time = avg_distance * 0.1 * (1.0 - avg_absorption);
            reverb.wet_level = reverb.room_size * 0.5;
            reverb.dry_level = 1.0 - reverb.wet_level * 0.3;
            reverb.diffusion = 1.0 - avg_absorption;
        }
        
        self.reverb = reverb.clone();
        self.stats.reverb_calculated += 1;
        
        reverb
    }
    
    /// Get effective volume for source at listener
    pub fn get_effective_volume(&self, source_id: u32) -> f32 {
        let source = match self.sources.get(&source_id) {
            Some(s) => s,
            None => return 0.0,
        };
        
        let distance = (self.listener_pos - source.position).length();
        
        if distance > source.max_distance {
            return 0.0;
        }
        
        // Distance attenuation
        let attenuation = 1.0 - (distance / source.max_distance);
        
        // Direction factor (louder in front)
        let to_source = (source.position - self.listener_pos).normalize();
        let direction_factor = (self.listener_dir.dot(to_source) * 0.25 + 0.75).max(0.5);
        
        source.volume * attenuation * direction_factor
    }
    
    /// Get panning for 3D audio
    pub fn get_panning(&self, source_id: u32) -> f32 {
        let source = match self.sources.get(&source_id) {
            Some(s) => s,
            None => return 0.0,
        };
        
        let to_source = (source.position - self.listener_pos).normalize();
        
        // Calculate right vector from listener direction
        let up = Vec3::Y;
        let right = self.listener_dir.cross(up).normalize();
        
        // Dot product with right gives left/right pan
        right.dot(to_source)
    }
    
    /// Get statistics
    pub fn get_stats(&self) -> AudioStats {
        self.stats.clone()
    }
    
    /// Clear cache
    pub fn clear_cache(&mut self) {
        self.occlusion_cache.clear();
    }
    
    /// Clear all
    pub fn clear(&mut self) {
        self.sources.clear();
        self.occlusion_cache.clear();
        self.stats = AudioStats::default();
    }
}
