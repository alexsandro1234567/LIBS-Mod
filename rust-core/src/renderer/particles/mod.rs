//! # GPU Particle System
//! 
//! High-performance GPU-driven particle system using compute shaders.
//! Supports millions of particles with physics simulation on GPU.

pub mod emitter;
pub mod simulation;
pub mod renderer;

use std::sync::Arc;
use ash::vk;

use super::vulkan::{VulkanDevice, VulkanError, Buffer, BufferType};

pub use emitter::*;
pub use simulation::*;
pub use renderer::*;

/// Maximum particles per system
pub const MAX_PARTICLES: usize = 1_000_000;

/// Particle data structure (GPU-side)
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Particle {
    /// Position (xyz) and size (w)
    pub position_size: [f32; 4],
    /// Velocity (xyz) and lifetime (w)
    pub velocity_lifetime: [f32; 4],
    /// Color (rgba)
    pub color: [f32; 4],
    /// Rotation (x), angular velocity (y), texture index (z), flags (w)
    pub rotation_tex_flags: [f32; 4],
}

/// Particle system configuration
#[derive(Debug, Clone)]
pub struct ParticleSystemConfig {
    /// Maximum number of particles
    pub max_particles: usize,
    /// Enable GPU simulation
    pub gpu_simulation: bool,
    /// Enable sorting for transparency
    pub sort_particles: bool,
    /// Particle texture atlas
    pub texture_atlas: Option<u32>,
    /// Simulation timestep
    pub timestep: f32,
}

impl Default for ParticleSystemConfig {
    fn default() -> Self {
        Self {
            max_particles: 100_000,
            gpu_simulation: true,
            sort_particles: true,
            texture_atlas: None,
            timestep: 1.0 / 60.0,
        }
    }
}

/// GPU Particle System
pub struct ParticleSystem {
    /// Device reference
    device: Arc<VulkanDevice>,
    /// Configuration
    config: ParticleSystemConfig,
    /// Particle buffer (double-buffered for simulation)
    particle_buffers: [Buffer; 2],
    /// Current buffer index
    current_buffer: usize,
    /// Alive particle count buffer
    count_buffer: Buffer,
    /// Indirect draw buffer
    indirect_buffer: Buffer,
    /// Emitter data buffer
    emitter_buffer: Buffer,
    /// Simulation compute pipeline
    simulation_pipeline: vk::Pipeline,
    /// Emission compute pipeline
    emission_pipeline: vk::Pipeline,
    /// Sort compute pipeline
    sort_pipeline: Option<vk::Pipeline>,
    /// Render pipeline
    render_pipeline: vk::Pipeline,
    /// Pipeline layout
    pipeline_layout: vk::PipelineLayout,
    /// Descriptor set layout
    descriptor_layout: vk::DescriptorSetLayout,
    /// Descriptor pool
    descriptor_pool: vk::DescriptorPool,
    /// Descriptor sets
    descriptor_sets: Vec<vk::DescriptorSet>,
    /// Active emitters
    emitters: Vec<ParticleEmitter>,
    /// Current particle count
    particle_count: u32,
    /// Accumulated time
    accumulated_time: f32,
}

impl ParticleSystem {
    /// Create a new particle system
    pub fn new(
        device: Arc<VulkanDevice>,
        config: ParticleSystemConfig,
    ) -> Result<Self, VulkanError> {
        let buffer_size = (config.max_particles * std::mem::size_of::<Particle>()) as u64;
        
        // Create particle buffers (double-buffered)
        let particle_buffer_0 = Buffer::new(device.clone(), buffer_size, BufferType::Storage)?;
        let particle_buffer_1 = Buffer::new(device.clone(), buffer_size, BufferType::Storage)?;
        
        // Create count buffer
        let count_buffer = Buffer::new(device.clone(), 16, BufferType::Storage)?;
        
        // Create indirect draw buffer
        let indirect_buffer = Buffer::new(
            device.clone(),
            std::mem::size_of::<vk::DrawIndirectCommand>() as u64,
            BufferType::Storage,
        )?;
        
        // Create emitter buffer
        let emitter_buffer = Buffer::new(
            device.clone(),
            (256 * std::mem::size_of::<EmitterData>()) as u64,
            BufferType::Storage,
        )?;
        
        // Create descriptor set layout
        let descriptor_layout = Self::create_descriptor_layout(&device)?;
        
        // Create pipeline layout
        let push_constant_range = vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::COMPUTE | vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)
            .offset(0)
            .size(64);
        
        let layouts = [descriptor_layout];
        let push_ranges = [push_constant_range];
        
        let layout_info = vk::PipelineLayoutCreateInfo::default()
            .set_layouts(&layouts)
            .push_constant_ranges(&push_ranges);
        
        let pipeline_layout = unsafe {
            device.handle().create_pipeline_layout(&layout_info, None)
                .map_err(|e| VulkanError::PipelineCreationFailed(format!("{:?}", e)))?
        };
        
        // Create descriptor pool
        let pool_sizes = [
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(16),
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(4),
        ];
        
        let pool_info = vk::DescriptorPoolCreateInfo::default()
            .max_sets(8)
            .pool_sizes(&pool_sizes);
        
        let descriptor_pool = unsafe {
            device.handle().create_descriptor_pool(&pool_info, None)
                .map_err(|e| VulkanError::PipelineCreationFailed(format!("{:?}", e)))?
        };
        
        // Create pipelines (would load actual shaders)
        let simulation_pipeline = vk::Pipeline::null();
        let emission_pipeline = vk::Pipeline::null();
        let sort_pipeline = if config.sort_particles { Some(vk::Pipeline::null()) } else { None };
        let render_pipeline = vk::Pipeline::null();
        
        Ok(Self {
            device,
            config,
            particle_buffers: [particle_buffer_0, particle_buffer_1],
            current_buffer: 0,
            count_buffer,
            indirect_buffer,
            emitter_buffer,
            simulation_pipeline,
            emission_pipeline,
            sort_pipeline,
            render_pipeline,
            pipeline_layout,
            descriptor_layout,
            descriptor_pool,
            descriptor_sets: Vec::new(),
            emitters: Vec::new(),
            particle_count: 0,
            accumulated_time: 0.0,
        })
    }
    
    /// Create descriptor set layout
    fn create_descriptor_layout(device: &VulkanDevice) -> Result<vk::DescriptorSetLayout, VulkanError> {
        let bindings = [
            // Binding 0: Input particle buffer
            vk::DescriptorSetLayoutBinding::default()
                .binding(0)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::COMPUTE | vk::ShaderStageFlags::VERTEX),
            // Binding 1: Output particle buffer
            vk::DescriptorSetLayoutBinding::default()
                .binding(1)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::COMPUTE),
            // Binding 2: Count buffer
            vk::DescriptorSetLayoutBinding::default()
                .binding(2)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::COMPUTE | vk::ShaderStageFlags::VERTEX),
            // Binding 3: Emitter buffer
            vk::DescriptorSetLayoutBinding::default()
                .binding(3)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::COMPUTE),
            // Binding 4: Indirect draw buffer
            vk::DescriptorSetLayoutBinding::default()
                .binding(4)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::COMPUTE),
            // Binding 5: Particle texture atlas
            vk::DescriptorSetLayoutBinding::default()
                .binding(5)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT),
        ];
        
        let layout_info = vk::DescriptorSetLayoutCreateInfo::default()
            .bindings(&bindings);
        
        unsafe {
            device.handle().create_descriptor_set_layout(&layout_info, None)
                .map_err(|e| VulkanError::PipelineCreationFailed(format!("{:?}", e)))
        }
    }
    
    /// Add an emitter
    pub fn add_emitter(&mut self, emitter: ParticleEmitter) -> usize {
        let id = self.emitters.len();
        self.emitters.push(emitter);
        id
    }
    
    /// Remove an emitter
    pub fn remove_emitter(&mut self, id: usize) {
        if id < self.emitters.len() {
            self.emitters.remove(id);
        }
    }
    
    /// Update emitter position
    pub fn set_emitter_position(&mut self, id: usize, x: f32, y: f32, z: f32) {
        if let Some(emitter) = self.emitters.get_mut(id) {
            emitter.position = [x, y, z];
        }
    }
    
    /// Update the particle system
    pub fn update(&mut self, delta_time: f32) {
        self.accumulated_time += delta_time;
        
        // Update emitters
        for emitter in &mut self.emitters {
            emitter.update(delta_time);
        }
    }
    
    /// Record simulation commands
    pub fn record_simulation(&self, cmd: vk::CommandBuffer) {
        if self.simulation_pipeline == vk::Pipeline::null() {
            return;
        }
        
        unsafe {
            // Bind simulation pipeline
            self.device.handle().cmd_bind_pipeline(
                cmd,
                vk::PipelineBindPoint::COMPUTE,
                self.simulation_pipeline,
            );
            
            // Dispatch simulation
            let workgroup_size = 256;
            let num_workgroups = (self.particle_count as u32 + workgroup_size - 1) / workgroup_size;
            self.device.handle().cmd_dispatch(cmd, num_workgroups, 1, 1);
            
            // Memory barrier
            let barrier = vk::MemoryBarrier::default()
                .src_access_mask(vk::AccessFlags::SHADER_WRITE)
                .dst_access_mask(vk::AccessFlags::SHADER_READ);
            
            self.device.handle().cmd_pipeline_barrier(
                cmd,
                vk::PipelineStageFlags::COMPUTE_SHADER,
                vk::PipelineStageFlags::COMPUTE_SHADER | vk::PipelineStageFlags::VERTEX_SHADER,
                vk::DependencyFlags::empty(),
                &[barrier],
                &[],
                &[],
            );
        }
    }
    
    /// Record emission commands
    pub fn record_emission(&self, cmd: vk::CommandBuffer) {
        if self.emission_pipeline == vk::Pipeline::null() || self.emitters.is_empty() {
            return;
        }
        
        unsafe {
            // Bind emission pipeline
            self.device.handle().cmd_bind_pipeline(
                cmd,
                vk::PipelineBindPoint::COMPUTE,
                self.emission_pipeline,
            );
            
            // Dispatch emission for each emitter
            for (i, emitter) in self.emitters.iter().enumerate() {
                if emitter.particles_to_emit > 0 {
                    let workgroup_size = 64;
                    let num_workgroups = (emitter.particles_to_emit + workgroup_size - 1) / workgroup_size;
                    self.device.handle().cmd_dispatch(cmd, num_workgroups, 1, 1);
                }
            }
        }
    }
    
    /// Record sort commands (for transparency)
    pub fn record_sort(&self, cmd: vk::CommandBuffer, camera_pos: [f32; 3]) {
        if let Some(sort_pipeline) = self.sort_pipeline {
            if sort_pipeline == vk::Pipeline::null() {
                return;
            }
            
            // Would implement GPU radix sort or bitonic sort
            // for sorting particles by distance to camera
        }
    }
    
    /// Record render commands
    pub fn record_render(&self, cmd: vk::CommandBuffer) {
        if self.render_pipeline == vk::Pipeline::null() || self.particle_count == 0 {
            return;
        }
        
        unsafe {
            // Bind render pipeline
            self.device.handle().cmd_bind_pipeline(
                cmd,
                vk::PipelineBindPoint::GRAPHICS,
                self.render_pipeline,
            );
            
            // Draw particles using indirect command
            self.device.handle().cmd_draw_indirect(
                cmd,
                self.indirect_buffer.handle(),
                0,
                1,
                std::mem::size_of::<vk::DrawIndirectCommand>() as u32,
            );
        }
    }
    
    /// Get current particle count
    pub fn particle_count(&self) -> u32 {
        self.particle_count
    }
    
    /// Get emitter count
    pub fn emitter_count(&self) -> usize {
        self.emitters.len()
    }
    
    /// Swap particle buffers
    fn swap_buffers(&mut self) {
        self.current_buffer = 1 - self.current_buffer;
    }
}

impl Drop for ParticleSystem {
    fn drop(&mut self) {
        unsafe {
            if self.simulation_pipeline != vk::Pipeline::null() {
                self.device.handle().destroy_pipeline(self.simulation_pipeline, None);
            }
            if self.emission_pipeline != vk::Pipeline::null() {
                self.device.handle().destroy_pipeline(self.emission_pipeline, None);
            }
            if let Some(sort_pipeline) = self.sort_pipeline {
                if sort_pipeline != vk::Pipeline::null() {
                    self.device.handle().destroy_pipeline(sort_pipeline, None);
                }
            }
            if self.render_pipeline != vk::Pipeline::null() {
                self.device.handle().destroy_pipeline(self.render_pipeline, None);
            }
            self.device.handle().destroy_pipeline_layout(self.pipeline_layout, None);
            self.device.handle().destroy_descriptor_pool(self.descriptor_pool, None);
            self.device.handle().destroy_descriptor_set_layout(self.descriptor_layout, None);
        }
    }
}

/// Emitter data for GPU (matches shader struct)
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct EmitterData {
    /// Emitter position
    pub position: [f32; 4],
    /// Emission direction
    pub direction: [f32; 4],
    /// Velocity range (min xyz, max w)
    pub velocity_min: [f32; 4],
    pub velocity_max: [f32; 4],
    /// Size range
    pub size_min: f32,
    pub size_max: f32,
    /// Lifetime range
    pub lifetime_min: f32,
    pub lifetime_max: f32,
    /// Color start
    pub color_start: [f32; 4],
    /// Color end
    pub color_end: [f32; 4],
    /// Emission rate
    pub rate: f32,
    /// Spread angle (radians)
    pub spread: f32,
    /// Gravity scale
    pub gravity: f32,
    /// Drag coefficient
    pub drag: f32,
}
