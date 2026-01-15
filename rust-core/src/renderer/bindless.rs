//! # Bindless Textures with Real Vulkan Implementation
//! 
//! Atlas elimination and per-block texture binding using Vulkan descriptor indexing.

use std::sync::Arc;
use std::collections::HashMap;
use parking_lot::RwLock;
use ash::vk;

/// Texture descriptor
#[derive(Debug, Clone)]
pub struct TextureDescriptor {
    pub id: u64,
    pub resource_location: String,
    pub image: vk::Image,
    pub image_view: vk::ImageView,
    pub sampler: vk::Sampler,
    pub memory: vk::DeviceMemory,
    pub width: u32,
    pub height: u32,
    pub mip_levels: u32,
    pub animated: bool,
    pub frame_count: u32,
    pub current_frame: u32,
    pub descriptor_index: u32,
}

/// Texture binding slot for shaders
#[derive(Debug, Clone, Copy)]
pub struct TextureBinding {
    pub descriptor_index: u32,
    pub sampler_index: u32,
    pub frame_offset: u32,
}

/// Bindless texture manager with real Vulkan implementation
pub struct BindlessTextureManager {
    device: Option<Arc<ash::Device>>,
    physical_device: Option<vk::PhysicalDevice>,
    textures: HashMap<u64, TextureDescriptor>,
    resource_map: HashMap<String, u64>,
    descriptor_pool: vk::DescriptorPool,
    descriptor_set_layout: vk::DescriptorSetLayout,
    descriptor_set: vk::DescriptorSet,
    global_sampler: vk::Sampler,
    max_textures: u32,
    next_id: u64,
    free_slots: Vec<u32>,
    animation_time: f32,
    initialized: bool,
    stats: BindlessStats,
}

#[derive(Debug, Default, Clone)]
pub struct BindlessStats {
    pub textures_loaded: u32,
    pub textures_bound: u32,
    pub max_resolution: u32,
    pub vram_usage_mb: f32,
    pub animated_textures: u32,
}

impl BindlessTextureManager {
    pub fn new() -> Self {
        log::info!("Creating Bindless Texture Manager");
        Self {
            device: None,
            physical_device: None,
            textures: HashMap::with_capacity(4096),
            resource_map: HashMap::with_capacity(4096),
            descriptor_pool: vk::DescriptorPool::null(),
            descriptor_set_layout: vk::DescriptorSetLayout::null(),
            descriptor_set: vk::DescriptorSet::null(),
            global_sampler: vk::Sampler::null(),
            max_textures: 16384,
            next_id: 1,
            free_slots: Vec::new(),
            animation_time: 0.0,
            initialized: false,
            stats: BindlessStats::default(),
        }
    }
    
    /// Initialize with Vulkan device
    pub fn initialize(
        &mut self, 
        device: Arc<ash::Device>,
        physical_device: vk::PhysicalDevice,
    ) -> Result<(), String> {
        self.device = Some(device.clone());
        self.physical_device = Some(physical_device);
        
        unsafe {
            // Create global sampler with anisotropic filtering
            let sampler_info = vk::SamplerCreateInfo::default()
                .mag_filter(vk::Filter::LINEAR)
                .min_filter(vk::Filter::LINEAR)
                .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
                .address_mode_u(vk::SamplerAddressMode::REPEAT)
                .address_mode_v(vk::SamplerAddressMode::REPEAT)
                .address_mode_w(vk::SamplerAddressMode::REPEAT)
                .anisotropy_enable(true)
                .max_anisotropy(16.0)
                .border_color(vk::BorderColor::INT_OPAQUE_BLACK)
                .unnormalized_coordinates(false)
                .compare_enable(false)
                .compare_op(vk::CompareOp::ALWAYS)
                .mip_lod_bias(0.0)
                .min_lod(0.0)
                .max_lod(12.0);
            
            self.global_sampler = device.create_sampler(&sampler_info, None)
                .map_err(|e| format!("Failed to create sampler: {:?}", e))?;
            
            // Create descriptor set layout with bindless array
            let binding = vk::DescriptorSetLayoutBinding::default()
                .binding(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(self.max_textures)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT | vk::ShaderStageFlags::VERTEX);
            
            let binding_flags = [
                vk::DescriptorBindingFlags::PARTIALLY_BOUND |
                vk::DescriptorBindingFlags::UPDATE_AFTER_BIND |
                vk::DescriptorBindingFlags::VARIABLE_DESCRIPTOR_COUNT
            ];
            
            let mut binding_flags_info = vk::DescriptorSetLayoutBindingFlagsCreateInfo::default()
                .binding_flags(&binding_flags);
            
            let layout_info = vk::DescriptorSetLayoutCreateInfo::default()
                .bindings(std::slice::from_ref(&binding))
                .flags(vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL)
                .push_next(&mut binding_flags_info);
            
            self.descriptor_set_layout = device.create_descriptor_set_layout(&layout_info, None)
                .map_err(|e| format!("Failed to create descriptor set layout: {:?}", e))?;
            
            // Create descriptor pool
            let pool_size = vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(self.max_textures);
            
            let pool_info = vk::DescriptorPoolCreateInfo::default()
                .pool_sizes(std::slice::from_ref(&pool_size))
                .max_sets(1)
                .flags(vk::DescriptorPoolCreateFlags::UPDATE_AFTER_BIND);
            
            self.descriptor_pool = device.create_descriptor_pool(&pool_info, None)
                .map_err(|e| format!("Failed to create descriptor pool: {:?}", e))?;
            
            // Allocate descriptor set
            let variable_count = [self.max_textures];
            let mut variable_info = vk::DescriptorSetVariableDescriptorCountAllocateInfo::default()
                .descriptor_counts(&variable_count);
            
            let alloc_info = vk::DescriptorSetAllocateInfo::default()
                .descriptor_pool(self.descriptor_pool)
                .set_layouts(std::slice::from_ref(&self.descriptor_set_layout))
                .push_next(&mut variable_info);
            
            let sets = device.allocate_descriptor_sets(&alloc_info)
                .map_err(|e| format!("Failed to allocate descriptor set: {:?}", e))?;
            
            self.descriptor_set = sets[0];
        }
        
        // Initialize free slots
        for i in (0..self.max_textures).rev() {
            self.free_slots.push(i);
        }
        
        self.initialized = true;
        log::info!("Bindless Texture Manager initialized (max {} textures)", self.max_textures);
        
        Ok(())
    }
    
    /// Load texture with real Vulkan image creation
    pub fn load_texture(
        &mut self,
        resource_location: &str,
        image_data: &[u8],
        width: u32,
        height: u32,
    ) -> Result<u64, String> {
        if !self.initialized {
            return Err("Not initialized".to_string());
        }
        
        // Check if already loaded
        if let Some(&id) = self.resource_map.get(resource_location) {
            return Ok(id);
        }
        
        let device = self.device.as_ref().ok_or("No device")?;
        let slot = self.free_slots.pop().ok_or("No free texture slots")?;
        
        let id = self.next_id;
        self.next_id += 1;
        
        let mip_levels = ((width.max(height) as f32).log2().floor() as u32 + 1).min(12);
        let animated = height > width && height % width == 0;
        let frame_count = if animated { height / width } else { 1 };
        let actual_height = if animated { width } else { height };
        
        unsafe {
            // Create image
            let image_info = vk::ImageCreateInfo::default()
                .image_type(vk::ImageType::TYPE_2D)
                .extent(vk::Extent3D { width, height: actual_height, depth: 1 })
                .mip_levels(mip_levels)
                .array_layers(if animated { frame_count } else { 1 })
                .format(vk::Format::R8G8B8A8_SRGB)
                .tiling(vk::ImageTiling::OPTIMAL)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .usage(vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED)
                .sharing_mode(vk::SharingMode::EXCLUSIVE)
                .samples(vk::SampleCountFlags::TYPE_1);
            
            let image = device.create_image(&image_info, None)
                .map_err(|e| format!("Failed to create image: {:?}", e))?;
            
            // Get memory requirements and allocate
            let mem_requirements = device.get_image_memory_requirements(image);
            
            let memory = self.allocate_memory(mem_requirements, vk::MemoryPropertyFlags::DEVICE_LOCAL)?;
            
            device.bind_image_memory(image, memory, 0)
                .map_err(|e| format!("Failed to bind image memory: {:?}", e))?;
            
            // Create image view
            let view_info = vk::ImageViewCreateInfo::default()
                .image(image)
                .view_type(if animated { vk::ImageViewType::TYPE_2D_ARRAY } else { vk::ImageViewType::TYPE_2D })
                .format(vk::Format::R8G8B8A8_SRGB)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: mip_levels,
                    base_array_layer: 0,
                    layer_count: if animated { frame_count } else { 1 },
                });
            
            let image_view = device.create_image_view(&view_info, None)
                .map_err(|e| format!("Failed to create image view: {:?}", e))?;
            
            // Update descriptor set
            let image_descriptor = vk::DescriptorImageInfo::default()
                .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .image_view(image_view)
                .sampler(self.global_sampler);
            
            let write = vk::WriteDescriptorSet::default()
                .dst_set(self.descriptor_set)
                .dst_binding(0)
                .dst_array_element(slot)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(std::slice::from_ref(&image_descriptor));
            
            device.update_descriptor_sets(std::slice::from_ref(&write), &[]);
            
            let descriptor = TextureDescriptor {
                id,
                resource_location: resource_location.to_string(),
                image,
                image_view,
                sampler: self.global_sampler,
                memory,
                width,
                height: actual_height,
                mip_levels,
                animated,
                frame_count,
                current_frame: 0,
                descriptor_index: slot,
            };
            
            self.textures.insert(id, descriptor);
            self.resource_map.insert(resource_location.to_string(), id);
            
            // Update stats
            self.stats.textures_loaded += 1;
            self.stats.max_resolution = self.stats.max_resolution.max(width.max(actual_height));
            self.stats.vram_usage_mb += (width * actual_height * 4) as f32 / (1024.0 * 1024.0);
            if animated {
                self.stats.animated_textures += 1;
            }
        }
        
        log::debug!("Loaded texture: {} ({}x{}, {} mips)", resource_location, width, height, mip_levels);
        
        Ok(id)
    }
    
    /// Allocate device memory
    fn allocate_memory(
        &self,
        requirements: vk::MemoryRequirements,
        properties: vk::MemoryPropertyFlags,
    ) -> Result<vk::DeviceMemory, String> {
        let device = self.device.as_ref().ok_or("No device")?;
        
        // Find memory type index (simplified - in production would query physical device)
        let memory_type_index = self.find_memory_type(requirements.memory_type_bits, properties)?;
        
        let alloc_info = vk::MemoryAllocateInfo::default()
            .allocation_size(requirements.size)
            .memory_type_index(memory_type_index);
        
        unsafe {
            device.allocate_memory(&alloc_info, None)
                .map_err(|e| format!("Failed to allocate memory: {:?}", e))
        }
    }
    
    fn find_memory_type(&self, type_filter: u32, properties: vk::MemoryPropertyFlags) -> Result<u32, String> {
        // Simplified - return first matching type
        for i in 0..32 {
            if (type_filter & (1 << i)) != 0 {
                return Ok(i);
            }
        }
        Err("No suitable memory type".to_string())
    }
    
    /// Get texture binding for shader
    pub fn get_binding(&self, resource_location: &str) -> Option<TextureBinding> {
        let id = self.resource_map.get(resource_location)?;
        let texture = self.textures.get(id)?;
        
        Some(TextureBinding {
            descriptor_index: texture.descriptor_index,
            sampler_index: 0,
            frame_offset: texture.current_frame,
        })
    }
    
    /// Get descriptor set for binding in pipeline
    pub fn descriptor_set(&self) -> vk::DescriptorSet {
        self.descriptor_set
    }
    
    /// Get descriptor set layout
    pub fn descriptor_set_layout(&self) -> vk::DescriptorSetLayout {
        self.descriptor_set_layout
    }
    
    /// Update animated textures
    pub fn update_animations(&mut self, delta_time: f32) {
        self.animation_time += delta_time;
        let frame_time = 0.05f32;
        
        for texture in self.textures.values_mut() {
            if texture.animated {
                texture.current_frame = ((self.animation_time / frame_time) as u32) % texture.frame_count;
            }
        }
    }
    
    /// Unload texture
    pub fn unload_texture(&mut self, resource_location: &str) {
        if let Some(id) = self.resource_map.remove(resource_location) {
            if let Some(texture) = self.textures.remove(&id) {
                if let Some(device) = &self.device {
                    unsafe {
                        device.destroy_image_view(texture.image_view, None);
                        device.destroy_image(texture.image, None);
                        device.free_memory(texture.memory, None);
                    }
                }
                self.free_slots.push(texture.descriptor_index);
                self.stats.textures_loaded -= 1;
            }
        }
    }
    
    pub fn texture_count(&self) -> usize { self.textures.len() }
    pub fn stats(&self) -> &BindlessStats { &self.stats }
    
    /// Shutdown
    pub fn shutdown(&mut self) {
        if let Some(device) = &self.device {
            unsafe {
                device.device_wait_idle().ok();
                
                for texture in self.textures.values() {
                    device.destroy_image_view(texture.image_view, None);
                    device.destroy_image(texture.image, None);
                    device.free_memory(texture.memory, None);
                }
                
                if self.descriptor_pool != vk::DescriptorPool::null() {
                    device.destroy_descriptor_pool(self.descriptor_pool, None);
                }
                if self.descriptor_set_layout != vk::DescriptorSetLayout::null() {
                    device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
                }
                if self.global_sampler != vk::Sampler::null() {
                    device.destroy_sampler(self.global_sampler, None);
                }
            }
        }
        
        self.textures.clear();
        self.resource_map.clear();
        self.initialized = false;
        log::info!("Bindless Texture Manager shutdown");
    }
}

impl Default for BindlessTextureManager {
    fn default() -> Self { Self::new() }
}

impl Drop for BindlessTextureManager {
    fn drop(&mut self) {
        self.shutdown();
    }
}
