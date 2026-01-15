//! # Particle Emitter
//! 
//! Particle emitter configuration and management.

use super::EmitterData;

/// Particle emitter
#[derive(Debug, Clone)]
pub struct ParticleEmitter {
    /// Emitter position
    pub position: [f32; 3],
    /// Emission direction
    pub direction: [f32; 3],
    /// Emission rate (particles per second)
    pub rate: f32,
    /// Velocity range
    pub velocity_min: [f32; 3],
    pub velocity_max: [f32; 3],
    /// Size range
    pub size_min: f32,
    pub size_max: f32,
    /// Lifetime range (seconds)
    pub lifetime_min: f32,
    pub lifetime_max: f32,
    /// Start color
    pub color_start: [f32; 4],
    /// End color
    pub color_end: [f32; 4],
    /// Spread angle (radians)
    pub spread: f32,
    /// Gravity scale
    pub gravity: f32,
    /// Drag coefficient
    pub drag: f32,
    /// Texture index in atlas
    pub texture_index: u32,
    /// Emitter shape
    pub shape: EmitterShape,
    /// Is emitter active
    pub active: bool,
    /// Accumulated time for emission
    pub accumulator: f32,
    /// Particles to emit this frame
    pub particles_to_emit: u32,
    /// Total particles emitted
    pub total_emitted: u64,
    /// Burst mode
    pub burst: Option<BurstConfig>,
}

impl Default for ParticleEmitter {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            direction: [0.0, 1.0, 0.0],
            rate: 100.0,
            velocity_min: [-1.0, 1.0, -1.0],
            velocity_max: [1.0, 3.0, 1.0],
            size_min: 0.1,
            size_max: 0.2,
            lifetime_min: 1.0,
            lifetime_max: 2.0,
            color_start: [1.0, 1.0, 1.0, 1.0],
            color_end: [1.0, 1.0, 1.0, 0.0],
            spread: 0.5,
            gravity: 1.0,
            drag: 0.02,
            texture_index: 0,
            shape: EmitterShape::Point,
            active: true,
            accumulator: 0.0,
            particles_to_emit: 0,
            total_emitted: 0,
            burst: None,
        }
    }
}

impl ParticleEmitter {
    /// Create a new point emitter
    pub fn point(position: [f32; 3]) -> Self {
        Self {
            position,
            shape: EmitterShape::Point,
            ..Default::default()
        }
    }
    
    /// Create a sphere emitter
    pub fn sphere(position: [f32; 3], radius: f32) -> Self {
        Self {
            position,
            shape: EmitterShape::Sphere { radius },
            ..Default::default()
        }
    }
    
    /// Create a box emitter
    pub fn box_emitter(position: [f32; 3], half_extents: [f32; 3]) -> Self {
        Self {
            position,
            shape: EmitterShape::Box { half_extents },
            ..Default::default()
        }
    }
    
    /// Create a cone emitter
    pub fn cone(position: [f32; 3], direction: [f32; 3], angle: f32, length: f32) -> Self {
        Self {
            position,
            direction,
            shape: EmitterShape::Cone { angle, length },
            ..Default::default()
        }
    }
    
    /// Set emission rate
    pub fn with_rate(mut self, rate: f32) -> Self {
        self.rate = rate;
        self
    }
    
    /// Set velocity range
    pub fn with_velocity(mut self, min: [f32; 3], max: [f32; 3]) -> Self {
        self.velocity_min = min;
        self.velocity_max = max;
        self
    }
    
    /// Set size range
    pub fn with_size(mut self, min: f32, max: f32) -> Self {
        self.size_min = min;
        self.size_max = max;
        self
    }
    
    /// Set lifetime range
    pub fn with_lifetime(mut self, min: f32, max: f32) -> Self {
        self.lifetime_min = min;
        self.lifetime_max = max;
        self
    }
    
    /// Set color gradient
    pub fn with_color(mut self, start: [f32; 4], end: [f32; 4]) -> Self {
        self.color_start = start;
        self.color_end = end;
        self
    }
    
    /// Set gravity scale
    pub fn with_gravity(mut self, gravity: f32) -> Self {
        self.gravity = gravity;
        self
    }
    
    /// Set drag coefficient
    pub fn with_drag(mut self, drag: f32) -> Self {
        self.drag = drag;
        self
    }
    
    /// Set texture index
    pub fn with_texture(mut self, index: u32) -> Self {
        self.texture_index = index;
        self
    }
    
    /// Enable burst mode
    pub fn with_burst(mut self, count: u32, interval: f32) -> Self {
        self.burst = Some(BurstConfig {
            count,
            interval,
            timer: 0.0,
        });
        self
    }
    
    /// Update emitter
    pub fn update(&mut self, delta_time: f32) {
        if !self.active {
            self.particles_to_emit = 0;
            return;
        }
        
        // Handle burst mode
        if let Some(ref mut burst) = self.burst {
            burst.timer += delta_time;
            if burst.timer >= burst.interval {
                burst.timer = 0.0;
                self.particles_to_emit = burst.count;
                self.total_emitted += burst.count as u64;
                return;
            }
        }
        
        // Continuous emission
        self.accumulator += delta_time * self.rate;
        self.particles_to_emit = self.accumulator as u32;
        self.accumulator -= self.particles_to_emit as f32;
        self.total_emitted += self.particles_to_emit as u64;
    }
    
    /// Convert to GPU data
    pub fn to_gpu_data(&self) -> EmitterData {
        EmitterData {
            position: [self.position[0], self.position[1], self.position[2], 0.0],
            direction: [self.direction[0], self.direction[1], self.direction[2], 0.0],
            velocity_min: [self.velocity_min[0], self.velocity_min[1], self.velocity_min[2], 0.0],
            velocity_max: [self.velocity_max[0], self.velocity_max[1], self.velocity_max[2], 0.0],
            size_min: self.size_min,
            size_max: self.size_max,
            lifetime_min: self.lifetime_min,
            lifetime_max: self.lifetime_max,
            color_start: self.color_start,
            color_end: self.color_end,
            rate: self.rate,
            spread: self.spread,
            gravity: self.gravity,
            drag: self.drag,
        }
    }
    
    /// Start emitting
    pub fn start(&mut self) {
        self.active = true;
    }
    
    /// Stop emitting
    pub fn stop(&mut self) {
        self.active = false;
        self.particles_to_emit = 0;
    }
    
    /// Reset emitter
    pub fn reset(&mut self) {
        self.accumulator = 0.0;
        self.particles_to_emit = 0;
        self.total_emitted = 0;
        if let Some(ref mut burst) = self.burst {
            burst.timer = 0.0;
        }
    }
}

/// Emitter shape
#[derive(Debug, Clone)]
pub enum EmitterShape {
    /// Point emitter
    Point,
    /// Sphere emitter
    Sphere { radius: f32 },
    /// Box emitter
    Box { half_extents: [f32; 3] },
    /// Cone emitter
    Cone { angle: f32, length: f32 },
    /// Circle emitter (2D)
    Circle { radius: f32 },
    /// Line emitter
    Line { start: [f32; 3], end: [f32; 3] },
    /// Mesh surface emitter
    Mesh { mesh_id: u32 },
}

/// Burst configuration
#[derive(Debug, Clone)]
pub struct BurstConfig {
    /// Number of particles per burst
    pub count: u32,
    /// Time between bursts
    pub interval: f32,
    /// Current timer
    pub timer: f32,
}

/// Particle effect preset
#[derive(Debug, Clone)]
pub enum ParticlePreset {
    Fire,
    Smoke,
    Explosion,
    Sparks,
    Rain,
    Snow,
    Dust,
    Magic,
    Blood,
    Bubbles,
}

impl ParticlePreset {
    /// Create emitter from preset
    pub fn create_emitter(&self, position: [f32; 3]) -> ParticleEmitter {
        match self {
            ParticlePreset::Fire => ParticleEmitter::cone(position, [0.0, 1.0, 0.0], 0.3, 2.0)
                .with_rate(200.0)
                .with_velocity([0.0, 2.0, 0.0], [0.0, 4.0, 0.0])
                .with_size(0.1, 0.3)
                .with_lifetime(0.5, 1.0)
                .with_color([1.0, 0.5, 0.0, 1.0], [1.0, 0.0, 0.0, 0.0])
                .with_gravity(-0.5)
                .with_texture(0),
            
            ParticlePreset::Smoke => ParticleEmitter::cone(position, [0.0, 1.0, 0.0], 0.5, 3.0)
                .with_rate(50.0)
                .with_velocity([0.0, 1.0, 0.0], [0.0, 2.0, 0.0])
                .with_size(0.2, 0.5)
                .with_lifetime(2.0, 4.0)
                .with_color([0.3, 0.3, 0.3, 0.5], [0.5, 0.5, 0.5, 0.0])
                .with_gravity(-0.1)
                .with_texture(1),
            
            ParticlePreset::Explosion => ParticleEmitter::sphere(position, 0.5)
                .with_burst(500, 0.0)
                .with_velocity([-5.0, -5.0, -5.0], [5.0, 5.0, 5.0])
                .with_size(0.1, 0.3)
                .with_lifetime(0.5, 1.5)
                .with_color([1.0, 0.8, 0.0, 1.0], [1.0, 0.2, 0.0, 0.0])
                .with_gravity(1.0)
                .with_texture(2),
            
            ParticlePreset::Sparks => ParticleEmitter::point(position)
                .with_rate(100.0)
                .with_velocity([-2.0, 1.0, -2.0], [2.0, 4.0, 2.0])
                .with_size(0.02, 0.05)
                .with_lifetime(0.3, 0.8)
                .with_color([1.0, 0.9, 0.5, 1.0], [1.0, 0.5, 0.0, 0.0])
                .with_gravity(2.0)
                .with_texture(3),
            
            ParticlePreset::Rain => ParticleEmitter::box_emitter(position, [50.0, 0.0, 50.0])
                .with_rate(1000.0)
                .with_velocity([0.0, -10.0, 0.0], [0.0, -15.0, 0.0])
                .with_size(0.01, 0.02)
                .with_lifetime(2.0, 3.0)
                .with_color([0.7, 0.8, 1.0, 0.5], [0.7, 0.8, 1.0, 0.0])
                .with_gravity(0.0)
                .with_texture(4),
            
            ParticlePreset::Snow => ParticleEmitter::box_emitter(position, [50.0, 0.0, 50.0])
                .with_rate(200.0)
                .with_velocity([-0.5, -1.0, -0.5], [0.5, -2.0, 0.5])
                .with_size(0.02, 0.05)
                .with_lifetime(5.0, 10.0)
                .with_color([1.0, 1.0, 1.0, 0.8], [1.0, 1.0, 1.0, 0.0])
                .with_gravity(0.0)
                .with_drag(0.1)
                .with_texture(5),
            
            ParticlePreset::Dust => ParticleEmitter::sphere(position, 1.0)
                .with_rate(30.0)
                .with_velocity([-0.5, 0.0, -0.5], [0.5, 0.5, 0.5])
                .with_size(0.05, 0.15)
                .with_lifetime(3.0, 6.0)
                .with_color([0.6, 0.5, 0.4, 0.3], [0.6, 0.5, 0.4, 0.0])
                .with_gravity(-0.05)
                .with_texture(6),
            
            ParticlePreset::Magic => ParticleEmitter::sphere(position, 0.5)
                .with_rate(50.0)
                .with_velocity([-1.0, -1.0, -1.0], [1.0, 1.0, 1.0])
                .with_size(0.05, 0.1)
                .with_lifetime(1.0, 2.0)
                .with_color([0.5, 0.0, 1.0, 1.0], [0.0, 1.0, 1.0, 0.0])
                .with_gravity(0.0)
                .with_texture(7),
            
            ParticlePreset::Blood => ParticleEmitter::point(position)
                .with_burst(50, 0.0)
                .with_velocity([-3.0, 1.0, -3.0], [3.0, 4.0, 3.0])
                .with_size(0.02, 0.08)
                .with_lifetime(0.5, 1.5)
                .with_color([0.5, 0.0, 0.0, 1.0], [0.3, 0.0, 0.0, 0.0])
                .with_gravity(3.0)
                .with_texture(8),
            
            ParticlePreset::Bubbles => ParticleEmitter::point(position)
                .with_rate(20.0)
                .with_velocity([-0.2, 0.5, -0.2], [0.2, 1.5, 0.2])
                .with_size(0.02, 0.1)
                .with_lifetime(2.0, 4.0)
                .with_color([0.8, 0.9, 1.0, 0.5], [0.8, 0.9, 1.0, 0.0])
                .with_gravity(-0.5)
                .with_texture(9),
        }
    }
}
