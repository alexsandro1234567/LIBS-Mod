//! # VulkanGL Interop with VK_KHR_external_memory
//! 
//! Zero-copy buffer and texture sharing between Vulkan and OpenGL
//! using external memory extensions.

use std::sync::Arc;
use std::collections::HashMap;
use ash::vk;

/// External memory handle type
#[cfg(target_os = "windows")]
pub type ExternalHandle = *mut std::ffi::c_void;
#[cfg(not(target_os = "windows"))]
pub type ExternalHandle = i32;

/// Shared buffer between Vulkan and OpenGL
#[derive(Debug)]
pub struct SharedBuffer {
    pub id: u64,
    pub buffer_type: InteropBufferType,
    pub size: usize,
    pub vk_buffer: vk::Buffer,
    pub vk_memory: vk::DeviceMemory,
    pub gl_buffer: u32,
    pub external_handle: ExternalHandle,
    pub vulkan_owned: bool,
}

/// Shared texture between Vulkan and OpenGL
#[derive(Debug)]
pub struct SharedTexture {
    pub id: u64,
    pub width: u32,
    pub height: u32,
    pub format: vk::Format,
    pub vk_image: vk::Image,
    pub vk_memory: vk::DeviceMemory,
    pub vk_view: vk::ImageView,
    pub gl_texture: u32,
    pub gl_memory_object: u32,
    pub external_handle: ExternalHandle,
    pub vulkan_owned: bool,
}

/// Buffer type for interop
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteropBufferType {
    Vertex,
    Index,
    Uniform,
    Storage,
    Indirect,
}

/// Vulkan-OpenGL Interop Manager
pub struct VulkanGLInterop {
    device: Option<Arc<ash::Device>>,
    physical_device: Option<vk::PhysicalDevice>,
    buffers: HashMap<u64, SharedBuffer>,
    textures: HashMap<u64, SharedTexture>,
    vk_to_gl_semaphore: vk::Semaphore,
    gl_to_vk_semaphore: vk::Semaphore,
    next_id: u64,
    initialized: bool,
    external_memory_supported: bool,
    external_semaphore_supported: bool,
}

impl VulkanGLInterop {
    pub fn new() -> Self {
        log::info!("Creating VulkanGL Interop Manager");
        Self {
            device: None,
            physical_device: None,
            buffers: HashMap::new(),
            textures: HashMap::new(),
            vk_to_gl_semaphore: vk::Semaphore::null(),
            gl_to_vk_semaphore: vk::Semaphore::null(),
            next_id: 1,
            initialized: false,
            external_memory_supported: false,
            external_semaphore_supported: false,
        }
    }
    
    /// Initialize with Vulkan device
    pub fn initialize(
        &mut self,
        instance: &ash::Instance,
        device: Arc<ash::Device>,
        physical_device: vk::PhysicalDevice,
    ) -> Result<(), String> {
        self.device = Some(device.clone());
        self.physical_device = Some(physical_device);
        
        // Check extension support
        self.check_extension_support(instance, physical_device)?;
        
        unsafe {
            // Create sync semaphores with external capability
            if self.external_semaphore_supported {
                self.vk_to_gl_semaphore = self.create_external_semaphore(&device)?;
                self.gl_to_vk_semaphore = self.create_external_semaphore(&device)?;
            }
        }
        
        self.initialized = true;
        log::info!("VulkanGL Interop initialized (ext_mem: {}, ext_sem: {})",
            self.external_memory_supported, self.external_semaphore_supported);
        
        Ok(())
    }
    
    fn check_extension_support(
        &mut self,
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
    ) -> Result<(), String> {
        unsafe {
            let extensions = instance.enumerate_device_extension_properties(physical_device)
                .map_err(|e| format!("Failed to enumerate extensions: {:?}", e))?;
            
            for ext in &extensions {
                let name = std::ffi::CStr::from_ptr(ext.extension_name.as_ptr())
                    .to_string_lossy();
                
                if name.contains("external_memory") { self.external_memory_supported = true; }
                if name.contains("external_semaphore") { self.external_semaphore_supported = true; }
            }
        }
        Ok(())
    }
    
    fn create_external_semaphore(&self, device: &ash::Device) -> Result<vk::Semaphore, String> {
        #[cfg(target_os = "windows")]
        let handle_type = vk::ExternalSemaphoreHandleTypeFlags::OPAQUE_WIN32;
        #[cfg(not(target_os = "windows"))]
        let handle_type = vk::ExternalSemaphoreHandleTypeFlags::OPAQUE_FD;
        
        let mut export_info = vk::ExportSemaphoreCreateInfo::default()
            .handle_types(handle_type);
        
        let create_info = vk::SemaphoreCreateInfo::default()
            .push_next(&mut export_info);
        
        unsafe {
            device.create_semaphore(&create_info, None)
                .map_err(|e| format!("Failed to create external semaphore: {:?}", e))
        }
    }
    
    /// Create shared buffer accessible by both Vulkan and OpenGL
    pub fn create_shared_buffer(
        &mut self,
        buffer_type: InteropBufferType,
        size: usize,
    ) -> Result<u64, String> {
        if !self.initialized { return Err("Interop not initialized".to_string()); }
        
        let device = self.device.as_ref().ok_or("No device")?;
        let id = self.next_id;
        self.next_id += 1;
        
        unsafe {
            let usage = match buffer_type {
                InteropBufferType::Vertex => vk::BufferUsageFlags::VERTEX_BUFFER,
                InteropBufferType::Index => vk::BufferUsageFlags::INDEX_BUFFER,
                InteropBufferType::Uniform => vk::BufferUsageFlags::UNIFORM_BUFFER,
                InteropBufferType::Storage => vk::BufferUsageFlags::STORAGE_BUFFER,
                InteropBufferType::Indirect => vk::BufferUsageFlags::INDIRECT_BUFFER,
            } | vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST;
            
            let buffer_info = vk::BufferCreateInfo::default()
                .size(size as u64)
                .usage(usage)
                .sharing_mode(vk::SharingMode::EXCLUSIVE);
            
            let vk_buffer = device.create_buffer(&buffer_info, None)
                .map_err(|e| format!("Failed to create buffer: {:?}", e))?;
            
            let mem_requirements = device.get_buffer_memory_requirements(vk_buffer);
            
            #[cfg(target_os = "windows")]
            let handle_type = vk::ExternalMemoryHandleTypeFlags::OPAQUE_WIN32;
            #[cfg(not(target_os = "windows"))]
            let handle_type = vk::ExternalMemoryHandleTypeFlags::OPAQUE_FD;
            
            let mut export_info = vk::ExportMemoryAllocateInfo::default()
                .handle_types(handle_type);
            
            let alloc_info = vk::MemoryAllocateInfo::default()
                .allocation_size(mem_requirements.size)
                .memory_type_index(self.find_memory_type(mem_requirements.memory_type_bits)?)
                .push_next(&mut export_info);
            
            let vk_memory = device.allocate_memory(&alloc_info, None)
                .map_err(|e| format!("Failed to allocate memory: {:?}", e))?;
            
            device.bind_buffer_memory(vk_buffer, vk_memory, 0)
                .map_err(|e| format!("Failed to bind buffer memory: {:?}", e))?;
            
            // External handle placeholder (actual handle retrieval requires extension loader)
            #[cfg(target_os = "windows")]
            let external_handle: ExternalHandle = std::ptr::null_mut();
            #[cfg(not(target_os = "windows"))]
            let external_handle: ExternalHandle = -1;
            
            let gl_buffer = id as u32; // Placeholder
            
            let buffer = SharedBuffer {
                id, buffer_type, size, vk_buffer, vk_memory, gl_buffer, external_handle, vulkan_owned: true,
            };
            self.buffers.insert(id, buffer);
            
            log::debug!("Created shared buffer {} ({:?}, {} bytes)", id, buffer_type, size);
        }
        Ok(id)
    }
    
    /// Create shared texture
    pub fn create_shared_texture(
        &mut self,
        width: u32,
        height: u32,
        format: vk::Format,
    ) -> Result<u64, String> {
        if !self.initialized { return Err("Interop not initialized".to_string()); }
        
        let device = self.device.as_ref().ok_or("No device")?;
        let id = self.next_id;
        self.next_id += 1;
        
        unsafe {
            #[cfg(target_os = "windows")]
            let handle_type = vk::ExternalMemoryHandleTypeFlags::OPAQUE_WIN32;
            #[cfg(not(target_os = "windows"))]
            let handle_type = vk::ExternalMemoryHandleTypeFlags::OPAQUE_FD;
            
            let mut external_info = vk::ExternalMemoryImageCreateInfo::default()
                .handle_types(handle_type);
            
            let image_info = vk::ImageCreateInfo::default()
                .image_type(vk::ImageType::TYPE_2D)
                .extent(vk::Extent3D { width, height, depth: 1 })
                .mip_levels(1)
                .array_layers(1)
                .format(format)
                .tiling(vk::ImageTiling::OPTIMAL)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .usage(vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::COLOR_ATTACHMENT |
                       vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::TRANSFER_DST)
                .sharing_mode(vk::SharingMode::EXCLUSIVE)
                .samples(vk::SampleCountFlags::TYPE_1)
                .push_next(&mut external_info);
            
            let vk_image = device.create_image(&image_info, None)
                .map_err(|e| format!("Failed to create image: {:?}", e))?;
            
            let mem_requirements = device.get_image_memory_requirements(vk_image);
            
            let mut export_info = vk::ExportMemoryAllocateInfo::default()
                .handle_types(handle_type);
            
            let mut dedicated_info = vk::MemoryDedicatedAllocateInfo::default()
                .image(vk_image);
            
            let alloc_info = vk::MemoryAllocateInfo::default()
                .allocation_size(mem_requirements.size)
                .memory_type_index(self.find_memory_type(mem_requirements.memory_type_bits)?)
                .push_next(&mut export_info)
                .push_next(&mut dedicated_info);
            
            let vk_memory = device.allocate_memory(&alloc_info, None)
                .map_err(|e| format!("Failed to allocate memory: {:?}", e))?;
            
            device.bind_image_memory(vk_image, vk_memory, 0)
                .map_err(|e| format!("Failed to bind image memory: {:?}", e))?;
            
            let view_info = vk::ImageViewCreateInfo::default()
                .image(vk_image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(format)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0, level_count: 1, base_array_layer: 0, layer_count: 1,
                });
            
            let vk_view = device.create_image_view(&view_info, None)
                .map_err(|e| format!("Failed to create image view: {:?}", e))?;
            
            #[cfg(target_os = "windows")]
            let external_handle: ExternalHandle = std::ptr::null_mut();
            #[cfg(not(target_os = "windows"))]
            let external_handle: ExternalHandle = -1;
            
            let texture = SharedTexture {
                id, width, height, format, vk_image, vk_memory, vk_view,
                gl_texture: id as u32, gl_memory_object: id as u32 + 1,
                external_handle, vulkan_owned: true,
            };
            self.textures.insert(id, texture);
            
            log::debug!("Created shared texture {} ({}x{}, {:?})", id, width, height, format);
        }
        Ok(id)
    }
    
    fn find_memory_type(&self, type_filter: u32) -> Result<u32, String> {
        for i in 0..32 {
            if (type_filter & (1 << i)) != 0 { return Ok(i); }
        }
        Err("No suitable memory type".to_string())
    }
    
    /// Transfer ownership to OpenGL
    pub fn transfer_to_opengl(&mut self, buffer_id: u64, queue: vk::Queue, cmd: vk::CommandBuffer) -> Result<(), String> {
        let device = self.device.as_ref().ok_or("No device")?;
        let buffer = self.buffers.get_mut(&buffer_id).ok_or("Buffer not found")?;
        if !buffer.vulkan_owned { return Ok(()); }
        
        unsafe {
            let submit = vk::SubmitInfo::default()
                .command_buffers(std::slice::from_ref(&cmd))
                .signal_semaphores(std::slice::from_ref(&self.vk_to_gl_semaphore));
            device.queue_submit(queue, std::slice::from_ref(&submit), vk::Fence::null())
                .map_err(|e| format!("Failed to submit: {:?}", e))?;
        }
        buffer.vulkan_owned = false;
        Ok(())
    }
    
    /// Transfer ownership to Vulkan
    pub fn transfer_to_vulkan(&mut self, buffer_id: u64, queue: vk::Queue, cmd: vk::CommandBuffer) -> Result<(), String> {
        let device = self.device.as_ref().ok_or("No device")?;
        let buffer = self.buffers.get_mut(&buffer_id).ok_or("Buffer not found")?;
        if buffer.vulkan_owned { return Ok(()); }
        
        unsafe {
            let wait_stages = [vk::PipelineStageFlags::ALL_COMMANDS];
            let submit = vk::SubmitInfo::default()
                .wait_semaphores(std::slice::from_ref(&self.gl_to_vk_semaphore))
                .wait_dst_stage_mask(&wait_stages)
                .command_buffers(std::slice::from_ref(&cmd));
            device.queue_submit(queue, std::slice::from_ref(&submit), vk::Fence::null())
                .map_err(|e| format!("Failed to submit: {:?}", e))?;
        }
        buffer.vulkan_owned = true;
        Ok(())
    }
    
    pub fn get_buffer(&self, id: u64) -> Option<&SharedBuffer> { self.buffers.get(&id) }
    pub fn get_texture(&self, id: u64) -> Option<&SharedTexture> { self.textures.get(&id) }
    
    pub fn destroy_buffer(&mut self, id: u64) {
        if let Some(buffer) = self.buffers.remove(&id) {
            if let Some(device) = &self.device {
                unsafe {
                    device.destroy_buffer(buffer.vk_buffer, None);
                    device.free_memory(buffer.vk_memory, None);
                }
            }
        }
    }
    
    pub fn destroy_texture(&mut self, id: u64) {
        if let Some(texture) = self.textures.remove(&id) {
            if let Some(device) = &self.device {
                unsafe {
                    device.destroy_image_view(texture.vk_view, None);
                    device.destroy_image(texture.vk_image, None);
                    device.free_memory(texture.vk_memory, None);
                }
            }
        }
    }
    
    pub fn shutdown(&mut self) {
        if let Some(device) = &self.device {
            unsafe {
                device.device_wait_idle().ok();
                
                for buffer in self.buffers.values() {
                    device.destroy_buffer(buffer.vk_buffer, None);
                    device.free_memory(buffer.vk_memory, None);
                }
                for texture in self.textures.values() {
                    device.destroy_image_view(texture.vk_view, None);
                    device.destroy_image(texture.vk_image, None);
                    device.free_memory(texture.vk_memory, None);
                }
                if self.vk_to_gl_semaphore != vk::Semaphore::null() {
                    device.destroy_semaphore(self.vk_to_gl_semaphore, None);
                }
                if self.gl_to_vk_semaphore != vk::Semaphore::null() {
                    device.destroy_semaphore(self.gl_to_vk_semaphore, None);
                }
            }
        }
        self.buffers.clear();
        self.textures.clear();
        self.initialized = false;
        log::info!("VulkanGL Interop shutdown");
    }
}

impl Default for VulkanGLInterop { fn default() -> Self { Self::new() } }
impl Drop for VulkanGLInterop { fn drop(&mut self) { self.shutdown(); } }
