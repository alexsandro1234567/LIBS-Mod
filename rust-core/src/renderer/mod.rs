//! # Renderer Module
//! 
//! Vulkan/OpenGL rendering system with mesh shaders, particles, and shader compilation.

pub mod vulkan;
pub mod shaders;
pub mod particles;
pub mod quantum;
pub mod bindless;

use std::collections::HashMap;
use crate::engine::EngineConfig;

/// The renderer
pub struct Renderer {
    /// Render mode
    mode: RenderMode,
    
    /// Current frame number
    frame: u64,
    
    /// In-frame flag
    in_frame: bool,
    
    /// Camera position
    camera_x: f64,
    camera_y: f64,
    camera_z: f64,
    camera_yaw: f32,
    camera_pitch: f32,
    
    /// Loaded textures
    textures: HashMap<u64, RendererTexture>,
    
    /// Shader manager
    shader_manager: Option<shaders::ShaderManager>,
    
    /// Particle systems
    particle_systems: Vec<particles::ParticleSystem>,
}

/// Render mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderMode {
    Vulkan,
    OpenGL,
    Hybrid,
}

/// Renderer texture info
#[derive(Debug)]
pub struct RendererTexture {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub format: u32,
    // In full implementation, would have Vulkan/OpenGL handles
}

impl Renderer {
    /// Create a new renderer
    pub fn new(config: &EngineConfig) -> Result<Self, String> {
        let mode = match config.render_mode {
            crate::engine::config::RenderMode::Vulkan => RenderMode::Vulkan,
            crate::engine::config::RenderMode::Opengl => RenderMode::OpenGL,
            crate::engine::config::RenderMode::Hybrid => RenderMode::Hybrid,
        };
        
        log::info!("Renderer created with mode: {:?}", mode);
        
        // Initialize shader manager
        let shader_manager = Some(shaders::ShaderManager::new(None));
        
        Ok(Self {
            mode,
            frame: 0,
            in_frame: false,
            camera_x: 0.0,
            camera_y: 0.0,
            camera_z: 0.0,
            camera_yaw: 0.0,
            camera_pitch: 0.0,
            textures: HashMap::new(),
            shader_manager,
            particle_systems: Vec::new(),
        })
    }
    
    /// Begin a frame
    pub fn begin_frame(&mut self) {
        self.in_frame = true;
        // In full implementation:
        // - Acquire swapchain image
        // - Begin command buffer
        // - Set up render pass
    }
    
    /// End a frame
    pub fn end_frame(&mut self) {
        self.in_frame = false;
        self.frame += 1;
        // In full implementation:
        // - End render pass
        // - Submit command buffer
        // - Present swapchain image
    }
    
    /// Set camera position
    pub fn set_camera(&mut self, x: f64, y: f64, z: f64, yaw: f32, pitch: f32) {
        self.camera_x = x;
        self.camera_y = y;
        self.camera_z = z;
        self.camera_yaw = yaw;
        self.camera_pitch = pitch;
    }
    
    /// Upload a texture
    pub fn upload_texture(&mut self, handle: u64, name: &str, _data: &[u8], width: u32, height: u32, format: u32) {
        let texture = RendererTexture {
            name: name.to_string(),
            width,
            height,
            format,
        };
        
        self.textures.insert(handle, texture);
        
        // In full implementation, would upload to GPU
        log::trace!("Renderer: Texture uploaded: {} ({}x{})", name, width, height);
    }
    
    /// Unload a texture
    pub fn unload_texture(&mut self, handle: u64) {
        if self.textures.remove(&handle).is_some() {
            // In full implementation, would free GPU resources
            log::trace!("Renderer: Texture unloaded: handle {}", handle);
        }
    }
    
    /// Get render mode
    pub fn mode(&self) -> RenderMode {
        self.mode
    }
    
    /// Get frame count
    pub fn frame_count(&self) -> u64 {
        self.frame
    }
    
    /// Get texture count
    pub fn texture_count(&self) -> usize {
        self.textures.len()
    }
    
    /// Get shader manager
    pub fn shader_manager(&self) -> Option<&shaders::ShaderManager> {
        self.shader_manager.as_ref()
    }
    
    /// Get mutable shader manager
    pub fn shader_manager_mut(&mut self) -> Option<&mut shaders::ShaderManager> {
        self.shader_manager.as_mut()
    }
    
    /// Update particle systems
    pub fn update_particles(&mut self, delta_time: f32) {
        for system in &mut self.particle_systems {
            system.update(delta_time);
        }
    }
}

/// Shutdown renderer subsystem
pub fn shutdown() {
    log::debug!("Renderer subsystem shutdown");
}
