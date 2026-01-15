//! # Particle Simulation
//! 
//! GPU-based particle physics simulation.

use super::Particle;

/// Simulation parameters
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SimulationParams {
    /// Delta time
    pub delta_time: f32,
    /// Gravity vector
    pub gravity: [f32; 3],
    /// Global wind
    pub wind: [f32; 3],
    /// Turbulence strength
    pub turbulence: f32,
    /// Turbulence frequency
    pub turbulence_frequency: f32,
    /// Current time (for noise)
    pub time: f32,
    /// Collision plane height (Y)
    pub floor_height: f32,
    /// Bounce factor
    pub bounce: f32,
    /// Friction
    pub friction: f32,
    /// Padding
    _padding: [f32; 3],
}

impl Default for SimulationParams {
    fn default() -> Self {
        Self {
            delta_time: 1.0 / 60.0,
            gravity: [0.0, -9.81, 0.0],
            wind: [0.0, 0.0, 0.0],
            turbulence: 0.0,
            turbulence_frequency: 1.0,
            time: 0.0,
            floor_height: 0.0,
            bounce: 0.5,
            friction: 0.8,
            _padding: [0.0; 3],
        }
    }
}

/// CPU fallback particle simulation
pub struct CpuParticleSimulator {
    /// Simulation parameters
    params: SimulationParams,
}

impl CpuParticleSimulator {
    /// Create a new CPU simulator
    pub fn new() -> Self {
        Self {
            params: SimulationParams::default(),
        }
    }
    
    /// Set simulation parameters
    pub fn set_params(&mut self, params: SimulationParams) {
        self.params = params;
    }
    
    /// Simulate particles
    pub fn simulate(&self, particles: &mut [Particle], alive_count: &mut u32) {
        let dt = self.params.delta_time;
        let gravity = self.params.gravity;
        
        let mut write_idx = 0;
        
        for i in 0..*alive_count as usize {
            let particle = &mut particles[i];
            
            // Decrease lifetime
            particle.velocity_lifetime[3] -= dt;
            
            // Skip dead particles
            if particle.velocity_lifetime[3] <= 0.0 {
                continue;
            }
            
            // Apply gravity
            particle.velocity_lifetime[0] += gravity[0] * dt;
            particle.velocity_lifetime[1] += gravity[1] * dt;
            particle.velocity_lifetime[2] += gravity[2] * dt;
            
            // Apply wind
            particle.velocity_lifetime[0] += self.params.wind[0] * dt;
            particle.velocity_lifetime[1] += self.params.wind[1] * dt;
            particle.velocity_lifetime[2] += self.params.wind[2] * dt;
            
            // Apply turbulence (simplified)
            if self.params.turbulence > 0.0 {
                let noise = self.simple_noise(
                    particle.position_size[0],
                    particle.position_size[1],
                    particle.position_size[2],
                    self.params.time,
                );
                particle.velocity_lifetime[0] += noise[0] * self.params.turbulence * dt;
                particle.velocity_lifetime[1] += noise[1] * self.params.turbulence * dt;
                particle.velocity_lifetime[2] += noise[2] * self.params.turbulence * dt;
            }
            
            // Update position
            particle.position_size[0] += particle.velocity_lifetime[0] * dt;
            particle.position_size[1] += particle.velocity_lifetime[1] * dt;
            particle.position_size[2] += particle.velocity_lifetime[2] * dt;
            
            // Floor collision
            if particle.position_size[1] < self.params.floor_height {
                particle.position_size[1] = self.params.floor_height;
                particle.velocity_lifetime[1] = -particle.velocity_lifetime[1] * self.params.bounce;
                particle.velocity_lifetime[0] *= self.params.friction;
                particle.velocity_lifetime[2] *= self.params.friction;
            }
            
            // Update rotation
            particle.rotation_tex_flags[0] += particle.rotation_tex_flags[1] * dt;
            
            // Compact alive particles
            if write_idx != i {
                particles[write_idx] = *particle;
            }
            write_idx += 1;
        }
        
        *alive_count = write_idx as u32;
    }
    
    /// Simple 3D noise for turbulence
    fn simple_noise(&self, x: f32, y: f32, z: f32, t: f32) -> [f32; 3] {
        let freq = self.params.turbulence_frequency;
        [
            (x * freq + t).sin() * (z * freq * 0.7).cos(),
            (y * freq + t * 1.3).sin() * (x * freq * 0.8).cos(),
            (z * freq + t * 0.9).sin() * (y * freq * 0.6).cos(),
        ]
    }
}

impl Default for CpuParticleSimulator {
    fn default() -> Self {
        Self::new()
    }
}

/// Particle collision shapes
#[derive(Debug, Clone)]
pub enum CollisionShape {
    /// Infinite plane
    Plane { normal: [f32; 3], distance: f32 },
    /// Sphere
    Sphere { center: [f32; 3], radius: f32 },
    /// Axis-aligned box
    Box { min: [f32; 3], max: [f32; 3] },
}

impl CollisionShape {
    /// Check collision and return penetration depth and normal
    pub fn check_collision(&self, position: [f32; 3], radius: f32) -> Option<([f32; 3], f32)> {
        match self {
            CollisionShape::Plane { normal, distance } => {
                let dot = position[0] * normal[0] + position[1] * normal[1] + position[2] * normal[2];
                let penetration = *distance - dot + radius;
                if penetration > 0.0 {
                    Some((*normal, penetration))
                } else {
                    None
                }
            }
            CollisionShape::Sphere { center, radius: sphere_radius } => {
                let dx = position[0] - center[0];
                let dy = position[1] - center[1];
                let dz = position[2] - center[2];
                let dist = (dx * dx + dy * dy + dz * dz).sqrt();
                let penetration = *sphere_radius + radius - dist;
                if penetration > 0.0 && dist > 0.0 {
                    let normal = [dx / dist, dy / dist, dz / dist];
                    Some((normal, penetration))
                } else {
                    None
                }
            }
            CollisionShape::Box { min, max } => {
                // Find closest point on box
                let closest = [
                    position[0].clamp(min[0], max[0]),
                    position[1].clamp(min[1], max[1]),
                    position[2].clamp(min[2], max[2]),
                ];
                
                let dx = position[0] - closest[0];
                let dy = position[1] - closest[1];
                let dz = position[2] - closest[2];
                let dist = (dx * dx + dy * dy + dz * dz).sqrt();
                
                if dist < radius {
                    let penetration = radius - dist;
                    if dist > 0.0 {
                        let normal = [dx / dist, dy / dist, dz / dist];
                        Some((normal, penetration))
                    } else {
                        // Inside box, push out along shortest axis
                        let dx_min = (position[0] - min[0]).abs();
                        let dx_max = (max[0] - position[0]).abs();
                        let dy_min = (position[1] - min[1]).abs();
                        let dy_max = (max[1] - position[1]).abs();
                        let dz_min = (position[2] - min[2]).abs();
                        let dz_max = (max[2] - position[2]).abs();
                        
                        let min_dist = dx_min.min(dx_max).min(dy_min).min(dy_max).min(dz_min).min(dz_max);
                        
                        let normal = if min_dist == dx_min { [-1.0, 0.0, 0.0] }
                        else if min_dist == dx_max { [1.0, 0.0, 0.0] }
                        else if min_dist == dy_min { [0.0, -1.0, 0.0] }
                        else if min_dist == dy_max { [0.0, 1.0, 0.0] }
                        else if min_dist == dz_min { [0.0, 0.0, -1.0] }
                        else { [0.0, 0.0, 1.0] };
                        
                        Some((normal, min_dist + radius))
                    }
                } else {
                    None
                }
            }
        }
    }
}

/// Force field for particle simulation
#[derive(Debug, Clone)]
pub enum ForceField {
    /// Directional force (like wind)
    Directional { direction: [f32; 3], strength: f32 },
    /// Point attractor/repulsor
    Point { center: [f32; 3], strength: f32, radius: f32 },
    /// Vortex
    Vortex { center: [f32; 3], axis: [f32; 3], strength: f32, radius: f32 },
    /// Turbulence
    Turbulence { strength: f32, frequency: f32, octaves: u32 },
    /// Drag
    Drag { coefficient: f32 },
}

impl ForceField {
    /// Calculate force at position
    pub fn calculate_force(&self, position: [f32; 3], velocity: [f32; 3], time: f32) -> [f32; 3] {
        match self {
            ForceField::Directional { direction, strength } => {
                [direction[0] * strength, direction[1] * strength, direction[2] * strength]
            }
            ForceField::Point { center, strength, radius } => {
                let dx = center[0] - position[0];
                let dy = center[1] - position[1];
                let dz = center[2] - position[2];
                let dist = (dx * dx + dy * dy + dz * dz).sqrt();
                
                if dist < *radius && dist > 0.0 {
                    let falloff = 1.0 - (dist / radius);
                    let force = strength * falloff / dist;
                    [dx * force, dy * force, dz * force]
                } else {
                    [0.0, 0.0, 0.0]
                }
            }
            ForceField::Vortex { center, axis, strength, radius } => {
                let dx = position[0] - center[0];
                let dy = position[1] - center[1];
                let dz = position[2] - center[2];
                let dist = (dx * dx + dy * dy + dz * dz).sqrt();
                
                if dist < *radius && dist > 0.0 {
                    // Cross product with axis
                    let cross = [
                        axis[1] * dz - axis[2] * dy,
                        axis[2] * dx - axis[0] * dz,
                        axis[0] * dy - axis[1] * dx,
                    ];
                    let falloff = 1.0 - (dist / radius);
                    let force = strength * falloff / dist;
                    [cross[0] * force, cross[1] * force, cross[2] * force]
                } else {
                    [0.0, 0.0, 0.0]
                }
            }
            ForceField::Turbulence { strength, frequency, octaves } => {
                // Simplified turbulence
                let mut force = [0.0f32; 3];
                let mut freq = *frequency;
                let mut amp = *strength;
                
                for _ in 0..*octaves {
                    force[0] += (position[0] * freq + time).sin() * amp;
                    force[1] += (position[1] * freq + time * 1.3).sin() * amp;
                    force[2] += (position[2] * freq + time * 0.7).sin() * amp;
                    freq *= 2.0;
                    amp *= 0.5;
                }
                
                force
            }
            ForceField::Drag { coefficient } => {
                let speed_sq = velocity[0] * velocity[0] + velocity[1] * velocity[1] + velocity[2] * velocity[2];
                if speed_sq > 0.0 {
                    let speed = speed_sq.sqrt();
                    let drag = -coefficient * speed;
                    [velocity[0] / speed * drag, velocity[1] / speed * drag, velocity[2] / speed * drag]
                } else {
                    [0.0, 0.0, 0.0]
                }
            }
        }
    }
}
