//! # ECS Systems
//! 
//! System implementations for processing entities.

use super::components::*;
use super::System;

/// Physics system - handles movement and gravity
pub struct PhysicsSystem {
    gravity: f64,
}

impl PhysicsSystem {
    pub fn new() -> Self {
        Self { gravity: -32.0 }
    }
}

impl Default for PhysicsSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl System for PhysicsSystem {
    fn name(&self) -> &'static str {
        "PhysicsSystem"
    }
    
    fn run(&mut self, world: &mut hecs::World, delta_time: f32) {
        let dt = delta_time as f64;
        
        // Update velocities with gravity and apply to positions
        for (_entity, (transform, velocity, physics)) in world.query_mut::<(&mut Transform, &mut Velocity, &PhysicsBody)>() {
            // Apply gravity
            if !physics.grounded {
                velocity.y += self.gravity * physics.gravity_scale as f64 * dt;
            }
            
            // Apply drag
            let drag = 1.0 - physics.drag as f64;
            velocity.x *= drag;
            velocity.z *= drag;
            
            // Update position
            transform.position.x += velocity.x * dt;
            transform.position.y += velocity.y * dt;
            transform.position.z += velocity.z * dt;
        }
        
        // Simple entities without physics body
        for (_entity, (transform, velocity)) in world.query_mut::<(&mut Transform, &Velocity)>()
            .without::<&PhysicsBody>()
        {
            transform.position.x += velocity.x * dt;
            transform.position.y += velocity.y * dt;
            transform.position.z += velocity.z * dt;
        }
    }
}

/// Transform system - handles rotation and scaling
pub struct TransformSystem;

impl TransformSystem {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TransformSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl System for TransformSystem {
    fn name(&self) -> &'static str {
        "TransformSystem"
    }
    
    fn run(&mut self, world: &mut hecs::World, delta_time: f32) {
        // Update rotations with angular velocity
        for (_entity, (transform, angular_vel)) in world.query_mut::<(&mut Transform, &AngularVelocity)>() {
            transform.rotation.yaw += angular_vel.yaw * delta_time;
            transform.rotation.pitch += angular_vel.pitch * delta_time;
            transform.rotation.roll += angular_vel.roll * delta_time;
            
            // Normalize angles
            transform.rotation.yaw = transform.rotation.yaw.rem_euclid(360.0);
            transform.rotation.pitch = transform.rotation.pitch.clamp(-90.0, 90.0);
        }
    }
}

/// Animation system - updates animation states
pub struct AnimationSystem;

impl AnimationSystem {
    pub fn new() -> Self {
        Self
    }
}

impl Default for AnimationSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl System for AnimationSystem {
    fn name(&self) -> &'static str {
        "AnimationSystem"
    }
    
    fn run(&mut self, world: &mut hecs::World, delta_time: f32) {
        for (_entity, animation) in world.query_mut::<&mut AnimationState>() {
            animation.time += delta_time * animation.speed;
            
            // In full implementation, would:
            // - Check animation duration
            // - Handle looping/non-looping
            // - Trigger animation events
            // - Blend between animations
        }
    }
}

/// Culling system - determines entity visibility
pub struct CullingSystem {
    camera_pos: Position,
    frustum_planes: [[f32; 4]; 6],
}

impl CullingSystem {
    pub fn new() -> Self {
        Self {
            camera_pos: Position::default(),
            frustum_planes: [[0.0; 4]; 6],
        }
    }
    
    /// Update camera position for distance culling
    pub fn set_camera_position(&mut self, x: f64, y: f64, z: f64) {
        self.camera_pos = Position { x, y, z };
    }
    
    /// Update frustum planes for frustum culling
    pub fn set_frustum(&mut self, view_projection: &[[f32; 4]; 4]) {
        // Extract frustum planes from view-projection matrix
        // Left plane
        self.frustum_planes[0] = [
            view_projection[0][3] + view_projection[0][0],
            view_projection[1][3] + view_projection[1][0],
            view_projection[2][3] + view_projection[2][0],
            view_projection[3][3] + view_projection[3][0],
        ];
        
        // Right plane
        self.frustum_planes[1] = [
            view_projection[0][3] - view_projection[0][0],
            view_projection[1][3] - view_projection[1][0],
            view_projection[2][3] - view_projection[2][0],
            view_projection[3][3] - view_projection[3][0],
        ];
        
        // Bottom plane
        self.frustum_planes[2] = [
            view_projection[0][3] + view_projection[0][1],
            view_projection[1][3] + view_projection[1][1],
            view_projection[2][3] + view_projection[2][1],
            view_projection[3][3] + view_projection[3][1],
        ];
        
        // Top plane
        self.frustum_planes[3] = [
            view_projection[0][3] - view_projection[0][1],
            view_projection[1][3] - view_projection[1][1],
            view_projection[2][3] - view_projection[2][1],
            view_projection[3][3] - view_projection[3][1],
        ];
        
        // Near plane
        self.frustum_planes[4] = [
            view_projection[0][3] + view_projection[0][2],
            view_projection[1][3] + view_projection[1][2],
            view_projection[2][3] + view_projection[2][2],
            view_projection[3][3] + view_projection[3][2],
        ];
        
        // Far plane
        self.frustum_planes[5] = [
            view_projection[0][3] - view_projection[0][2],
            view_projection[1][3] - view_projection[1][2],
            view_projection[2][3] - view_projection[2][2],
            view_projection[3][3] - view_projection[3][2],
        ];
        
        // Normalize planes
        for plane in &mut self.frustum_planes {
            let len = (plane[0] * plane[0] + plane[1] * plane[1] + plane[2] * plane[2]).sqrt();
            if len > 0.0 {
                plane[0] /= len;
                plane[1] /= len;
                plane[2] /= len;
                plane[3] /= len;
            }
        }
    }
    
    /// Check if a point is inside the frustum
    fn point_in_frustum(&self, x: f32, y: f32, z: f32) -> bool {
        for plane in &self.frustum_planes {
            if plane[0] * x + plane[1] * y + plane[2] * z + plane[3] < 0.0 {
                return false;
            }
        }
        true
    }
    
    /// Check if a sphere is inside or intersecting the frustum
    fn sphere_in_frustum(&self, x: f32, y: f32, z: f32, radius: f32) -> bool {
        for plane in &self.frustum_planes {
            if plane[0] * x + plane[1] * y + plane[2] * z + plane[3] < -radius {
                return false;
            }
        }
        true
    }
}

impl Default for CullingSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl System for CullingSystem {
    fn name(&self) -> &'static str {
        "CullingSystem"
    }
    
    fn run(&mut self, world: &mut hecs::World, _delta_time: f32) {
        for (_entity, (transform, visibility, bounds)) in world.query_mut::<(&Transform, &mut Visibility, &Bounds)>() {
            // Distance culling
            let distance = transform.position.distance(&self.camera_pos) as f32;
            if distance > visibility.render_distance {
                visibility.visible = false;
                continue;
            }
            
            // Frustum culling
            let x = transform.position.x as f32;
            let y = transform.position.y as f32;
            let z = transform.position.z as f32;
            let radius = bounds.half_extents[0].max(bounds.half_extents[1]).max(bounds.half_extents[2]);
            
            visibility.visible = self.sphere_in_frustum(x, y, z, radius);
        }
    }
}

/// AI system - handles entity AI behavior
pub struct AISystem;

impl AISystem {
    pub fn new() -> Self {
        Self
    }
}

impl Default for AISystem {
    fn default() -> Self {
        Self::new()
    }
}

impl System for AISystem {
    fn name(&self) -> &'static str {
        "AISystem"
    }
    
    fn run(&mut self, world: &mut hecs::World, delta_time: f32) {
        for (_entity, (transform, velocity, ai)) in world.query_mut::<(&Transform, &mut Velocity, &mut AIState)>() {
            match ai.behavior.as_str() {
                "idle" => {
                    // Do nothing
                }
                "wander" => {
                    // Random wandering behavior
                    // In full implementation, would pick random destinations
                }
                "follow_path" => {
                    // Follow pre-computed path
                    if ai.path_index < ai.path.len() {
                        let target = &ai.path[ai.path_index];
                        let dx = target.x - transform.position.x;
                        let dz = target.z - transform.position.z;
                        let dist = (dx * dx + dz * dz).sqrt();
                        
                        if dist < 0.5 {
                            ai.path_index += 1;
                        } else {
                            let speed = 4.0; // blocks per second
                            velocity.x = (dx / dist) * speed;
                            velocity.z = (dz / dist) * speed;
                        }
                    }
                }
                "chase" => {
                    // Chase target entity
                    // Would need to look up target position
                }
                _ => {}
            }
        }
    }
}

/// Health system - handles damage and death
pub struct HealthSystem;

impl HealthSystem {
    pub fn new() -> Self {
        Self
    }
}

impl Default for HealthSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl System for HealthSystem {
    fn name(&self) -> &'static str {
        "HealthSystem"
    }
    
    fn run(&mut self, world: &mut hecs::World, _delta_time: f32) {
        // Collect dead entities
        let dead: Vec<hecs::Entity> = world.query::<&Health>()
            .iter()
            .filter(|(_, health)| health.is_dead())
            .map(|(entity, _)| entity)
            .collect();
        
        // Handle death (would trigger events, spawn particles, etc.)
        for entity in dead {
            // In full implementation, would:
            // - Trigger death event
            // - Spawn death particles
            // - Drop items
            // - Play death sound
            // - Mark for despawn
            let _ = entity;
        }
    }
}

/// Particle system - spawns and updates particles
pub struct ParticleSystem {
    max_particles: usize,
}

impl ParticleSystem {
    pub fn new(max_particles: usize) -> Self {
        Self { max_particles }
    }
}

impl Default for ParticleSystem {
    fn default() -> Self {
        Self::new(10000)
    }
}

impl System for ParticleSystem {
    fn name(&self) -> &'static str {
        "ParticleSystem"
    }
    
    fn run(&mut self, world: &mut hecs::World, delta_time: f32) {
        for (_entity, (transform, emitter)) in world.query_mut::<(&Transform, &mut ParticleEmitter)>() {
            emitter.accumulator += delta_time * emitter.rate;
            
            while emitter.accumulator >= 1.0 {
                emitter.accumulator -= 1.0;
                
                // In full implementation, would spawn particle
                // with position = transform.position
                // and random velocity based on velocity_spread
            }
        }
    }
}
