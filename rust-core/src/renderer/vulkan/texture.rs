//! # Vulkan Texture
//! 
//! GPU texture and sampler management.

use std::sync::Arc;
use ash::vk;

use super::{VulkanDevice, VulkanError};

/// Texture format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureFormat {
    RGBA8,
    RGBA8Srgb,
    BGRA8,
    BGRA8Srgb,
    R8,
    RG8,
    R16F,
    RG16F,
    RGBA16F,
    R32F,
    RG32F,
    RGBA32F,
    Depth32F,
    Depth24Stencil8,
}

impl TextureFormat {
    /// Convert to Vulkan format
    pub fn to_vk(self) -> vk::Format {
        match self {
            TextureFormat::RGBA8 => vk::Format::R8G8B8A8_UNORM,
            TextureFormat::RGBA8Srgb => vk::Format::R8G8B8A8_SRGB,
            TextureFormat::BGRA8 => vk::Format::B8G8R8A8_UNORM,
            TextureFormat::BGRA8Srgb => vk::Format::B8G8R8A8_SRGB,
            TextureFormat::R8 => vk::Format::R8_UNORM,
            TextureFormat::RG8 => vk::Format::R8G8_UNORM,
            TextureFormat::R16F => vk::Format::R16_SFLOAT,
            TextureFormat::RG16F => vk::Format::R16G16_SFLOAT,
            TextureFormat::RGBA16F => vk::Format::R16G16B16A16_SFLOAT,
            TextureFormat::R32F => vk::Format::R32_SFLOAT,
            TextureFormat::RG32F => vk::Format::R32G32_SFLOAT,
            TextureFormat::RGBA32F => vk::Format::R32G32B32A32_SFLOAT,
            TextureFormat::Depth32F => vk::Format::D32_SFLOAT,
            TextureFormat::Depth24Stencil8 => vk::Format::D24_UNORM_S8_UINT,
        }
    }
    
    /// Get bytes per pixel
    pub fn bytes_per_pixel(self) -> u32 {
        match self {
            TextureFormat::R8 => 1,
            TextureFormat::RG8 | TextureFormat::R16F => 2,
            TextureFormat::RGBA8 | TextureFormat::RGBA8Srgb | 
            TextureFormat::BGRA8 | TextureFormat::BGRA8Srgb |
            TextureFormat::RG16F | TextureFormat::R32F |
            TextureFormat::Depth32F | TextureFormat::Depth24Stencil8 => 4,
            TextureFormat::RGBA16F | TextureFormat::RG32F => 8,
            TextureFormat::RGBA32F => 16,
        }
    }
}

/// Texture wrapper
pub struct Texture {
    /// Device reference
    device: Arc<VulkanDevice>,
    /// Image handle
    image: vk::Image,
    /// Image memory
    memory: vk::DeviceMemory,
    /// Image view
    view: vk::ImageView,
    /// Sampler
    sampler: vk::Sampler,
    /// Texture width
    width: u32,
    /// Texture height
    height: u32,
    /// Mip levels
    mip_levels: u32,
    /// Format
    format: TextureFormat,
}

impl Texture {
    /// Create a new texture
    pub fn new(
        device: Arc<VulkanDevice>,
        width: u32,
        height: u32,
        format: TextureFormat,
        generate_mipmaps: bool,
    ) -> Result<Self, VulkanError> {
        let mip_levels = if generate_mipmaps {
            ((width.max(height) as f32).log2().floor() as u32) + 1
        } else {
            1
        };
        
        // Create image
        let image_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(format.to_vk())
            .extent(vk::Extent3D { width, height, depth: 1 })
            .mip_levels(mip_levels)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::TRANSFER_SRC)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED);
        
        let image = unsafe {
            device.handle().create_image(&image_info, None)
                .map_err(|e| VulkanError::TextureCreationFailed(format!("Failed to create image: {:?}", e)))?
        };
        
        // Allocate memory
        let mem_requirements = unsafe { device.handle().get_image_memory_requirements(image) };
        
        let mem_type = device.find_memory_type(
            mem_requirements.memory_type_bits,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        ).ok_or_else(|| VulkanError::TextureCreationFailed("No suitable memory type".to_string()))?;
        
        let alloc_info = vk::MemoryAllocateInfo::default()
            .allocation_size(mem_requirements.size)
            .memory_type_index(mem_type);
        
        let memory = unsafe {
            device.handle().allocate_memory(&alloc_info, None)
                .map_err(|e| VulkanError::TextureCreationFailed(format!("Failed to allocate memory: {:?}", e)))?
        };
        
        // Bind memory
        unsafe {
            device.handle().bind_image_memory(image, memory, 0)
                .map_err(|e| VulkanError::TextureCreationFailed(format!("Failed to bind memory: {:?}", e)))?;
        }
        
        // Create image view
        let view_info = vk::ImageViewCreateInfo::default()
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(format.to_vk())
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: mip_levels,
                base_array_layer: 0,
                layer_count: 1,
            });
        
        let view = unsafe {
            device.handle().create_image_view(&view_info, None)
                .map_err(|e| VulkanError::TextureCreationFailed(format!("Failed to create image view: {:?}", e)))?
        };
        
        // Create sampler
        let sampler_info = vk::SamplerCreateInfo::default()
            .mag_filter(vk::Filter::LINEAR)
            .min_filter(vk::Filter::LINEAR)
            .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
            .address_mode_u(vk::SamplerAddressMode::REPEAT)
            .address_mode_v(vk::SamplerAddressMode::REPEAT)
            .address_mode_w(vk::SamplerAddressMode::REPEAT)
            .anisotropy_enable(true)
            .max_anisotropy(device.properties().limits.max_sampler_anisotropy)
            .border_color(vk::BorderColor::INT_OPAQUE_BLACK)
            .unnormalized_coordinates(false)
            .compare_enable(false)
            .compare_op(vk::CompareOp::ALWAYS)
            .mip_lod_bias(0.0)
            .min_lod(0.0)
            .max_lod(mip_levels as f32);
        
        let sampler = unsafe {
            device.handle().create_sampler(&sampler_info, None)
                .map_err(|e| VulkanError::TextureCreationFailed(format!("Failed to create sampler: {:?}", e)))?
        };
        
        Ok(Self {
            device,
            image,
            memory,
            view,
            sampler,
            width,
            height,
            mip_levels,
            format,
        })
    }
    
    /// Create texture from data
    pub fn from_data(
        device: Arc<VulkanDevice>,
        width: u32,
        height: u32,
        format: TextureFormat,
        data: &[u8],
        generate_mipmaps: bool,
    ) -> Result<Self, VulkanError> {
        let texture = Self::new(device.clone(), width, height, format, generate_mipmaps)?;
        
        // Would need to upload data via staging buffer
        // This is a simplified version
        
        Ok(texture)
    }
    
    /// Get image handle
    pub fn image(&self) -> vk::Image {
        self.image
    }
    
    /// Get image view
    pub fn view(&self) -> vk::ImageView {
        self.view
    }
    
    /// Get sampler
    pub fn sampler(&self) -> vk::Sampler {
        self.sampler
    }
    
    /// Get width
    pub fn width(&self) -> u32 {
        self.width
    }
    
    /// Get height
    pub fn height(&self) -> u32 {
        self.height
    }
    
    /// Get mip levels
    pub fn mip_levels(&self) -> u32 {
        self.mip_levels
    }
    
    /// Get format
    pub fn format(&self) -> TextureFormat {
        self.format
    }
    
    /// Get descriptor image info
    pub fn descriptor_info(&self) -> vk::DescriptorImageInfo {
        vk::DescriptorImageInfo::default()
            .sampler(self.sampler)
            .image_view(self.view)
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        unsafe {
            self.device.handle().destroy_sampler(self.sampler, None);
            self.device.handle().destroy_image_view(self.view, None);
            self.device.handle().destroy_image(self.image, None);
            self.device.handle().free_memory(self.memory, None);
        }
    }
}
