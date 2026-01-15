//! # Vulkan Buffer
//! 
//! GPU buffer management for vertices, indices, uniforms, and storage.

use std::sync::Arc;
use ash::vk;

use super::{VulkanDevice, VulkanError};

/// Buffer type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferType {
    /// Vertex buffer
    Vertex,
    /// Index buffer
    Index,
    /// Uniform buffer
    Uniform,
    /// Storage buffer
    Storage,
    /// Staging buffer (CPU -> GPU transfer)
    Staging,
}

/// GPU buffer wrapper
pub struct Buffer {
    /// Device reference
    device: Arc<VulkanDevice>,
    /// Buffer handle
    buffer: vk::Buffer,
    /// Memory handle
    memory: vk::DeviceMemory,
    /// Buffer size
    size: vk::DeviceSize,
    /// Buffer type
    buffer_type: BufferType,
    /// Mapped pointer (if persistently mapped)
    mapped_ptr: Option<*mut std::ffi::c_void>,
}

impl Buffer {
    /// Create a new buffer
    pub fn new(
        device: Arc<VulkanDevice>,
        size: vk::DeviceSize,
        buffer_type: BufferType,
    ) -> Result<Self, VulkanError> {
        let (usage, memory_flags) = match buffer_type {
            BufferType::Vertex => (
                vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
            ),
            BufferType::Index => (
                vk::BufferUsageFlags::INDEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
            ),
            BufferType::Uniform => (
                vk::BufferUsageFlags::UNIFORM_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            ),
            BufferType::Storage => (
                vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
            ),
            BufferType::Staging => (
                vk::BufferUsageFlags::TRANSFER_SRC,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            ),
        };
        
        Self::create_buffer(device, size, usage, memory_flags, buffer_type)
    }
    
    /// Create buffer with specific usage and memory flags
    fn create_buffer(
        device: Arc<VulkanDevice>,
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        memory_flags: vk::MemoryPropertyFlags,
        buffer_type: BufferType,
    ) -> Result<Self, VulkanError> {
        // Create buffer
        let buffer_info = vk::BufferCreateInfo::default()
            .size(size)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);
        
        let buffer = unsafe {
            device.handle().create_buffer(&buffer_info, None)
                .map_err(|e| VulkanError::BufferCreationFailed(format!("Failed to create buffer: {:?}", e)))?
        };
        
        // Get memory requirements
        let mem_requirements = unsafe { device.handle().get_buffer_memory_requirements(buffer) };
        
        // Find suitable memory type
        let mem_type = device.find_memory_type(mem_requirements.memory_type_bits, memory_flags)
            .ok_or_else(|| VulkanError::BufferCreationFailed("No suitable memory type found".to_string()))?;
        
        // Allocate memory
        let alloc_info = vk::MemoryAllocateInfo::default()
            .allocation_size(mem_requirements.size)
            .memory_type_index(mem_type);
        
        let memory = unsafe {
            device.handle().allocate_memory(&alloc_info, None)
                .map_err(|e| VulkanError::BufferCreationFailed(format!("Failed to allocate memory: {:?}", e)))?
        };
        
        // Bind memory to buffer
        unsafe {
            device.handle().bind_buffer_memory(buffer, memory, 0)
                .map_err(|e| VulkanError::BufferCreationFailed(format!("Failed to bind memory: {:?}", e)))?;
        }
        
        // Persistently map uniform and staging buffers
        let mapped_ptr = if memory_flags.contains(vk::MemoryPropertyFlags::HOST_VISIBLE) {
            unsafe {
                let ptr = device.handle().map_memory(memory, 0, size, vk::MemoryMapFlags::empty())
                    .map_err(|e| VulkanError::BufferCreationFailed(format!("Failed to map memory: {:?}", e)))?;
                Some(ptr)
            }
        } else {
            None
        };
        
        Ok(Self {
            device,
            buffer,
            memory,
            size,
            buffer_type,
            mapped_ptr,
        })
    }
    
    /// Write data to buffer (for mapped buffers)
    pub fn write<T: Copy>(&self, data: &[T]) -> Result<(), VulkanError> {
        let ptr = self.mapped_ptr
            .ok_or_else(|| VulkanError::BufferCreationFailed("Buffer is not mapped".to_string()))?;
        
        let data_size = std::mem::size_of_val(data) as vk::DeviceSize;
        if data_size > self.size {
            return Err(VulkanError::BufferCreationFailed("Data exceeds buffer size".to_string()));
        }
        
        unsafe {
            std::ptr::copy_nonoverlapping(data.as_ptr() as *const u8, ptr as *mut u8, data_size as usize);
        }
        
        Ok(())
    }
    
    /// Write raw bytes to buffer
    pub fn write_bytes(&self, data: &[u8]) -> Result<(), VulkanError> {
        let ptr = self.mapped_ptr
            .ok_or_else(|| VulkanError::BufferCreationFailed("Buffer is not mapped".to_string()))?;
        
        if data.len() as vk::DeviceSize > self.size {
            return Err(VulkanError::BufferCreationFailed("Data exceeds buffer size".to_string()));
        }
        
        unsafe {
            std::ptr::copy_nonoverlapping(data.as_ptr(), ptr as *mut u8, data.len());
        }
        
        Ok(())
    }
    
    /// Get buffer handle
    pub fn handle(&self) -> vk::Buffer {
        self.buffer
    }
    
    /// Get buffer size
    pub fn size(&self) -> vk::DeviceSize {
        self.size
    }
    
    /// Get buffer type
    pub fn buffer_type(&self) -> BufferType {
        self.buffer_type
    }
    
    /// Check if buffer is mapped
    pub fn is_mapped(&self) -> bool {
        self.mapped_ptr.is_some()
    }
    
    /// Get mapped pointer
    pub fn mapped_ptr(&self) -> Option<*mut std::ffi::c_void> {
        self.mapped_ptr
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe {
            if self.mapped_ptr.is_some() {
                self.device.handle().unmap_memory(self.memory);
            }
            self.device.handle().destroy_buffer(self.buffer, None);
            self.device.handle().free_memory(self.memory, None);
        }
    }
}

// Safety: Buffer is Send + Sync as long as we don't access mapped_ptr from multiple threads
// In practice, buffer writes should be synchronized externally
unsafe impl Send for Buffer {}
unsafe impl Sync for Buffer {}
