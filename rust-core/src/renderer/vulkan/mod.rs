//! # Vulkan Renderer Module
//! 
//! Complete Vulkan rendering implementation for Project Aether.
//! Provides high-performance GPU-accelerated rendering with modern features.

pub mod instance;
pub mod device;
pub mod swapchain;
pub mod pipeline;
pub mod buffer;
pub mod texture;
pub mod command;
pub mod sync;
pub mod mesh_shader;
pub mod interop;

use std::sync::Arc;
use ash::vk;

pub use instance::VulkanInstance;
pub use device::VulkanDevice;
pub use swapchain::Swapchain;
pub use pipeline::Pipeline;
pub use buffer::{Buffer, BufferType};
pub use texture::Texture;
pub use command::CommandPool;
pub use sync::SyncObjects;

/// Vulkan renderer configuration
#[derive(Debug, Clone)]
pub struct VulkanConfig {
    /// Application name
    pub app_name: String,
    /// Application version
    pub app_version: u32,
    /// Enable validation layers
    pub validation_enabled: bool,
    /// Preferred present mode
    pub preferred_present_mode: vk::PresentModeKHR,
    /// Max frames in flight
    pub max_frames_in_flight: u32,
    /// Enable mesh shaders if available
    pub mesh_shaders_enabled: bool,
    /// Enable ray tracing if available
    pub ray_tracing_enabled: bool,
}

impl Default for VulkanConfig {
    fn default() -> Self {
        Self {
            app_name: "Project Aether".to_string(),
            app_version: vk::make_api_version(0, 1, 0, 0),
            validation_enabled: cfg!(debug_assertions),
            preferred_present_mode: vk::PresentModeKHR::MAILBOX,
            max_frames_in_flight: 2,
            mesh_shaders_enabled: true,
            ray_tracing_enabled: false,
        }
    }
}

/// Main Vulkan renderer
pub struct VulkanRenderer {
    /// Vulkan instance
    instance: Arc<VulkanInstance>,
    /// Logical device
    device: Arc<VulkanDevice>,
    /// Swapchain
    swapchain: Option<Swapchain>,
    /// Graphics pipeline
    pipeline: Option<Pipeline>,
    /// Command pool
    command_pool: Option<CommandPool>,
    /// Synchronization objects
    sync: Option<SyncObjects>,
    /// Current frame index
    current_frame: usize,
    /// Configuration
    config: VulkanConfig,
    /// Is initialized
    initialized: bool,
}

impl VulkanRenderer {
    /// Create a new Vulkan renderer
    pub fn new(config: VulkanConfig) -> Result<Self, VulkanError> {
        log::info!("Creating Vulkan renderer...");
        
        // Create instance
        let instance = Arc::new(VulkanInstance::new(&config)?);
        log::info!("  Vulkan instance created");
        
        // Create device
        let device = Arc::new(VulkanDevice::new(instance.clone(), &config)?);
        log::info!("  Vulkan device created");
        log::info!("  GPU: {}", device.gpu_name());
        log::info!("  Mesh shaders: {}", device.supports_mesh_shaders());
        log::info!("  Ray tracing: {}", device.supports_ray_tracing());
        
        Ok(Self {
            instance,
            device,
            swapchain: None,
            pipeline: None,
            command_pool: None,
            sync: None,
            current_frame: 0,
            config,
            initialized: false,
        })
    }
    
    /// Initialize rendering resources
    pub fn initialize(&mut self, window_handle: u64, width: u32, height: u32) -> Result<(), VulkanError> {
        log::info!("Initializing Vulkan renderer ({}x{})...", width, height);
        
        // Create swapchain
        self.swapchain = Some(Swapchain::new(
            self.instance.clone(),
            self.device.clone(),
            window_handle,
            width,
            height,
            &self.config,
        )?);
        log::info!("  Swapchain created");
        
        // Create command pool
        self.command_pool = Some(CommandPool::new(self.device.clone())?);
        log::info!("  Command pool created");
        
        // Create sync objects
        self.sync = Some(SyncObjects::new(
            self.device.clone(),
            self.config.max_frames_in_flight as usize,
        )?);
        log::info!("  Sync objects created");
        
        // Create pipeline
        self.pipeline = Some(Pipeline::new(
            self.device.clone(),
            self.swapchain.as_ref().unwrap(),
            &self.config,
        )?);
        log::info!("  Graphics pipeline created");
        
        self.initialized = true;
        log::info!("Vulkan renderer initialized");
        
        Ok(())
    }
    
    /// Begin a new frame
    pub fn begin_frame(&mut self) -> Result<FrameContext, VulkanError> {
        if !self.initialized {
            return Err(VulkanError::NotInitialized);
        }
        
        let sync = self.sync.as_ref().unwrap();
        let swapchain = self.swapchain.as_ref().unwrap();
        
        // Wait for previous frame
        sync.wait_for_fence(self.current_frame)?;
        
        // Acquire next image
        let image_index = swapchain.acquire_next_image(sync.image_available(self.current_frame))?;
        
        // Reset fence
        sync.reset_fence(self.current_frame)?;
        
        Ok(FrameContext {
            frame_index: self.current_frame,
            image_index: image_index as usize,
        })
    }
    
    /// End the current frame
    pub fn end_frame(&mut self, ctx: FrameContext) -> Result<(), VulkanError> {
        let sync = self.sync.as_ref().unwrap();
        let swapchain = self.swapchain.as_ref().unwrap();
        
        // Submit command buffer
        // (In full implementation, would submit recorded commands)
        
        // Present
        swapchain.present(
            ctx.image_index as u32,
            sync.render_finished(ctx.frame_index),
        )?;
        
        // Advance frame
        self.current_frame = (self.current_frame + 1) % self.config.max_frames_in_flight as usize;
        
        Ok(())
    }
    
    /// Resize the swapchain
    pub fn resize(&mut self, width: u32, height: u32) -> Result<(), VulkanError> {
        if !self.initialized {
            return Err(VulkanError::NotInitialized);
        }
        
        // Wait for device idle
        self.device.wait_idle()?;
        
        // Recreate swapchain
        if let Some(ref mut swapchain) = self.swapchain {
            swapchain.recreate(width, height)?;
        }
        
        log::info!("Swapchain resized to {}x{}", width, height);
        
        Ok(())
    }
    
    /// Shutdown the renderer
    pub fn shutdown(&mut self) {
        if !self.initialized {
            return;
        }
        
        log::info!("Shutting down Vulkan renderer...");
        
        // Wait for device idle
        let _ = self.device.wait_idle();
        
        // Cleanup in reverse order
        self.pipeline = None;
        self.sync = None;
        self.command_pool = None;
        self.swapchain = None;
        
        self.initialized = false;
        log::info!("Vulkan renderer shutdown complete");
    }
    
    /// Check if mesh shaders are supported
    pub fn supports_mesh_shaders(&self) -> bool {
        self.device.supports_mesh_shaders()
    }
    
    /// Check if ray tracing is supported
    pub fn supports_ray_tracing(&self) -> bool {
        self.device.supports_ray_tracing()
    }
    
    /// Get device reference
    pub fn device(&self) -> &Arc<VulkanDevice> {
        &self.device
    }
}

impl Drop for VulkanRenderer {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Frame context for rendering
#[derive(Debug, Clone, Copy)]
pub struct FrameContext {
    /// Current frame index (for double/triple buffering)
    pub frame_index: usize,
    /// Swapchain image index
    pub image_index: usize,
}

/// Vulkan error types
#[derive(Debug, Clone)]
pub enum VulkanError {
    /// Instance creation failed
    InstanceCreationFailed(String),
    /// No suitable GPU found
    NoSuitableGpu,
    /// Device creation failed
    DeviceCreationFailed(String),
    /// Swapchain creation failed
    SwapchainCreationFailed(String),
    /// Pipeline creation failed
    PipelineCreationFailed(String),
    /// Buffer creation failed
    BufferCreationFailed(String),
    /// Texture creation failed
    TextureCreationFailed(String),
    /// Command buffer error
    CommandBufferError(String),
    /// Synchronization error
    SyncError(String),
    /// Renderer not initialized
    NotInitialized,
    /// Surface lost
    SurfaceLost,
    /// Out of date swapchain
    OutOfDate,
    /// Generic Vulkan error
    VkError(String),
}

impl std::fmt::Display for VulkanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VulkanError::InstanceCreationFailed(msg) => write!(f, "Instance creation failed: {}", msg),
            VulkanError::NoSuitableGpu => write!(f, "No suitable GPU found"),
            VulkanError::DeviceCreationFailed(msg) => write!(f, "Device creation failed: {}", msg),
            VulkanError::SwapchainCreationFailed(msg) => write!(f, "Swapchain creation failed: {}", msg),
            VulkanError::PipelineCreationFailed(msg) => write!(f, "Pipeline creation failed: {}", msg),
            VulkanError::BufferCreationFailed(msg) => write!(f, "Buffer creation failed: {}", msg),
            VulkanError::TextureCreationFailed(msg) => write!(f, "Texture creation failed: {}", msg),
            VulkanError::CommandBufferError(msg) => write!(f, "Command buffer error: {}", msg),
            VulkanError::SyncError(msg) => write!(f, "Synchronization error: {}", msg),
            VulkanError::NotInitialized => write!(f, "Renderer not initialized"),
            VulkanError::SurfaceLost => write!(f, "Surface lost"),
            VulkanError::OutOfDate => write!(f, "Swapchain out of date"),
            VulkanError::VkError(msg) => write!(f, "Vulkan error: {}", msg),
        }
    }
}

impl std::error::Error for VulkanError {}
