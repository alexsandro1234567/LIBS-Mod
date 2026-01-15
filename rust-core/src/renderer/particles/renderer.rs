//! # Particle Renderer
//! 
//! GPU-based particle rendering with billboarding and instancing.

use ash::vk;

/// Particle render mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParticleRenderMode {
    /// Camera-facing billboards
    Billboard,
    /// Velocity-aligned billboards
    VelocityAligned,
    /// Stretched billboards based on velocity
    Stretched,
    /// 3D mesh instances
    Mesh,
    /// Point sprites
    Points,
    /// Trail/ribbon particles
    Trail,
}

/// Particle blend mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParticleBlendMode {
    /// Alpha blending
    Alpha,
    /// Additive blending
    Additive,
    /// Multiplicative blending
    Multiply,
    /// Premultiplied alpha
    Premultiplied,
    /// No blending (opaque)
    Opaque,
}

impl ParticleBlendMode {
    /// Get Vulkan blend state
    pub fn to_vk_blend(&self) -> vk::PipelineColorBlendAttachmentState {
        match self {
            ParticleBlendMode::Alpha => vk::PipelineColorBlendAttachmentState::default()
                .blend_enable(true)
                .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
                .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                .color_blend_op(vk::BlendOp::ADD)
                .src_alpha_blend_factor(vk::BlendFactor::ONE)
                .dst_alpha_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                .alpha_blend_op(vk::BlendOp::ADD)
                .color_write_mask(vk::ColorComponentFlags::RGBA),
            
            ParticleBlendMode::Additive => vk::PipelineColorBlendAttachmentState::default()
                .blend_enable(true)
                .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
                .dst_color_blend_factor(vk::BlendFactor::ONE)
                .color_blend_op(vk::BlendOp::ADD)
                .src_alpha_blend_factor(vk::BlendFactor::ZERO)
                .dst_alpha_blend_factor(vk::BlendFactor::ONE)
                .alpha_blend_op(vk::BlendOp::ADD)
                .color_write_mask(vk::ColorComponentFlags::RGBA),
            
            ParticleBlendMode::Multiply => vk::PipelineColorBlendAttachmentState::default()
                .blend_enable(true)
                .src_color_blend_factor(vk::BlendFactor::DST_COLOR)
                .dst_color_blend_factor(vk::BlendFactor::ZERO)
                .color_blend_op(vk::BlendOp::ADD)
                .src_alpha_blend_factor(vk::BlendFactor::DST_ALPHA)
                .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
                .alpha_blend_op(vk::BlendOp::ADD)
                .color_write_mask(vk::ColorComponentFlags::RGBA),
            
            ParticleBlendMode::Premultiplied => vk::PipelineColorBlendAttachmentState::default()
                .blend_enable(true)
                .src_color_blend_factor(vk::BlendFactor::ONE)
                .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                .color_blend_op(vk::BlendOp::ADD)
                .src_alpha_blend_factor(vk::BlendFactor::ONE)
                .dst_alpha_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                .alpha_blend_op(vk::BlendOp::ADD)
                .color_write_mask(vk::ColorComponentFlags::RGBA),
            
            ParticleBlendMode::Opaque => vk::PipelineColorBlendAttachmentState::default()
                .blend_enable(false)
                .color_write_mask(vk::ColorComponentFlags::RGBA),
        }
    }
}

/// Particle render configuration
#[derive(Debug, Clone)]
pub struct ParticleRenderConfig {
    /// Render mode
    pub mode: ParticleRenderMode,
    /// Blend mode
    pub blend: ParticleBlendMode,
    /// Write to depth buffer
    pub depth_write: bool,
    /// Test against depth buffer
    pub depth_test: bool,
    /// Soft particles (fade near geometry)
    pub soft_particles: bool,
    /// Soft particle distance
    pub soft_distance: f32,
    /// Near fade distance
    pub near_fade: f32,
    /// Far fade distance
    pub far_fade: f32,
    /// Texture atlas columns
    pub atlas_columns: u32,
    /// Texture atlas rows
    pub atlas_rows: u32,
    /// Animation frames per second
    pub animation_fps: f32,
}

impl Default for ParticleRenderConfig {
    fn default() -> Self {
        Self {
            mode: ParticleRenderMode::Billboard,
            blend: ParticleBlendMode::Alpha,
            depth_write: false,
            depth_test: true,
            soft_particles: true,
            soft_distance: 0.5,
            near_fade: 0.1,
            far_fade: 100.0,
            atlas_columns: 1,
            atlas_rows: 1,
            animation_fps: 30.0,
        }
    }
}

/// Particle render push constants
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ParticleRenderPushConstants {
    /// View-projection matrix
    pub view_proj: [[f32; 4]; 4],
    /// Camera position
    pub camera_pos: [f32; 4],
    /// Camera right vector
    pub camera_right: [f32; 4],
    /// Camera up vector
    pub camera_up: [f32; 4],
    /// Time and config
    pub time_config: [f32; 4], // time, soft_distance, near_fade, far_fade
    /// Atlas config
    pub atlas_config: [f32; 4], // columns, rows, fps, unused
}

/// Particle vertex for rendering
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct ParticleVertex {
    /// Position
    pub position: [f32; 3],
    /// UV coordinates
    pub uv: [f32; 2],
    /// Color
    pub color: [f32; 4],
    /// Size
    pub size: f32,
    /// Rotation
    pub rotation: f32,
    /// Texture index
    pub tex_index: f32,
}

/// GPU particle sorter using bitonic sort
pub struct ParticleSorter {
    /// Sort pipeline
    pipeline: vk::Pipeline,
    /// Pipeline layout
    layout: vk::PipelineLayout,
    /// Workgroup size
    workgroup_size: u32,
}

impl ParticleSorter {
    /// Create a new particle sorter
    pub fn new() -> Self {
        Self {
            pipeline: vk::Pipeline::null(),
            layout: vk::PipelineLayout::null(),
            workgroup_size: 256,
        }
    }
    
    /// Sort particles by distance to camera
    pub fn sort(
        &self,
        cmd: vk::CommandBuffer,
        particle_buffer: vk::Buffer,
        count: u32,
        camera_pos: [f32; 3],
    ) {
        if self.pipeline == vk::Pipeline::null() {
            return;
        }
        
        // Would dispatch bitonic sort compute shader
        // Multiple passes for full sort
    }
}

impl Default for ParticleSorter {
    fn default() -> Self {
        Self::new()
    }
}

/// Trail particle data
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct TrailPoint {
    /// Position
    pub position: [f32; 3],
    /// Width
    pub width: f32,
    /// Color
    pub color: [f32; 4],
    /// Age (0-1)
    pub age: f32,
    /// Padding
    _padding: [f32; 3],
}

/// Trail particle system
pub struct TrailSystem {
    /// Maximum trail points
    max_points: usize,
    /// Trail points buffer
    points: Vec<TrailPoint>,
    /// Active trails
    trails: Vec<Trail>,
}

/// Individual trail
pub struct Trail {
    /// Trail ID
    pub id: u32,
    /// Start index in points buffer
    pub start_index: usize,
    /// Point count
    pub point_count: usize,
    /// Maximum points
    pub max_points: usize,
    /// Emission rate (points per second)
    pub emission_rate: f32,
    /// Point lifetime
    pub lifetime: f32,
    /// Width
    pub width: f32,
    /// Color
    pub color: [f32; 4],
    /// Accumulator
    accumulator: f32,
}

impl TrailSystem {
    /// Create a new trail system
    pub fn new(max_points: usize) -> Self {
        Self {
            max_points,
            points: vec![TrailPoint::default(); max_points],
            trails: Vec::new(),
        }
    }
    
    /// Add a new trail
    pub fn add_trail(&mut self, max_points: usize, emission_rate: f32, lifetime: f32) -> u32 {
        let id = self.trails.len() as u32;
        let start_index = self.trails.iter().map(|t| t.start_index + t.max_points).max().unwrap_or(0);
        
        if start_index + max_points > self.max_points {
            return u32::MAX; // No space
        }
        
        self.trails.push(Trail {
            id,
            start_index,
            point_count: 0,
            max_points,
            emission_rate,
            lifetime,
            width: 0.1,
            color: [1.0, 1.0, 1.0, 1.0],
            accumulator: 0.0,
        });
        
        id
    }
    
    /// Update trail with new position
    pub fn update_trail(&mut self, id: u32, position: [f32; 3], delta_time: f32) {
        if let Some(trail) = self.trails.iter_mut().find(|t| t.id == id) {
            // Age existing points
            for i in trail.start_index..(trail.start_index + trail.point_count) {
                self.points[i].age += delta_time / trail.lifetime;
            }
            
            // Remove dead points
            while trail.point_count > 0 {
                let last_idx = trail.start_index + trail.point_count - 1;
                if self.points[last_idx].age >= 1.0 {
                    trail.point_count -= 1;
                } else {
                    break;
                }
            }
            
            // Add new points
            trail.accumulator += delta_time * trail.emission_rate;
            while trail.accumulator >= 1.0 && trail.point_count < trail.max_points {
                trail.accumulator -= 1.0;
                
                // Shift points
                for i in (trail.start_index + 1..trail.start_index + trail.point_count + 1).rev() {
                    self.points[i] = self.points[i - 1];
                }
                
                // Add new point at start
                self.points[trail.start_index] = TrailPoint {
                    position,
                    width: trail.width,
                    color: trail.color,
                    age: 0.0,
                    _padding: [0.0; 3],
                };
                
                trail.point_count += 1;
            }
        }
    }
    
    /// Remove a trail
    pub fn remove_trail(&mut self, id: u32) {
        self.trails.retain(|t| t.id != id);
    }
}
