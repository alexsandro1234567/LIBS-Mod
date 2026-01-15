//! # GUI Compositor with Real Vulkan Implementation
//! 
//! Composites GUI elements on top of 3D scene using Vulkan render passes.

use std::sync::Arc;
use parking_lot::RwLock;
use ash::vk;

/// GUI Layer types  
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuiLayer {
    Hud, Container, Chat, Debug, ModOverlay, BlurBackground,
}

/// GUI Element
#[derive(Debug, Clone)]
pub struct GuiElement {
    pub layer: GuiLayer,
    pub x: f32, pub y: f32,
    pub width: f32, pub height: f32,
    pub opacity: f32,
    pub texture_id: u64,
    pub z_order: i32,
    pub blur_radius: f32,
    pub visible: bool,
}

impl GuiElement {
    pub fn new(layer: GuiLayer, x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { layer, x, y, width, height, opacity: 1.0, texture_id: 0, z_order: 0, blur_radius: 0.0, visible: true }
    }
    
    pub fn with_blur(mut self, radius: f32) -> Self { self.blur_radius = radius; self }
    pub fn with_opacity(mut self, opacity: f32) -> Self { self.opacity = opacity; self }
}

/// Compositor config
#[derive(Debug, Clone)]
pub struct CompositorConfig {
    pub blur_enabled: bool,
    pub blur_quality: u8,
    pub gui_antialiasing: bool,
    pub scale_factor: f32,
    pub width: u32,
    pub height: u32,
}

impl Default for CompositorConfig {
    fn default() -> Self {
        Self { blur_enabled: true, blur_quality: 2, gui_antialiasing: true, scale_factor: 1.0, width: 1920, height: 1080 }
    }
}

/// GUI Compositor with real Vulkan framebuffers
pub struct GuiCompositor {
    device: Option<Arc<ash::Device>>,
    config: CompositorConfig,
    elements: Vec<GuiElement>,
    
    // Vulkan resources
    render_pass: vk::RenderPass,
    framebuffer: vk::Framebuffer,
    blur_framebuffer: vk::Framebuffer,
    color_image: vk::Image,
    color_memory: vk::DeviceMemory,
    color_view: vk::ImageView,
    blur_image: vk::Image,
    blur_memory: vk::DeviceMemory,
    blur_view: vk::ImageView,
    sampler: vk::Sampler,
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    descriptor_set_layout: vk::DescriptorSetLayout,
    descriptor_pool: vk::DescriptorPool,
    descriptor_set: vk::DescriptorSet,
    command_pool: vk::CommandPool,
    command_buffer: vk::CommandBuffer,
    
    initialized: bool,
}

impl GuiCompositor {
    pub fn new() -> Self {
        log::debug!("Creating GUI Compositor");
        Self {
            device: None,
            config: CompositorConfig::default(),
            elements: Vec::with_capacity(256),
            render_pass: vk::RenderPass::null(),
            framebuffer: vk::Framebuffer::null(),
            blur_framebuffer: vk::Framebuffer::null(),
            color_image: vk::Image::null(),
            color_memory: vk::DeviceMemory::null(),
            color_view: vk::ImageView::null(),
            blur_image: vk::Image::null(),
            blur_memory: vk::DeviceMemory::null(),
            blur_view: vk::ImageView::null(),
            sampler: vk::Sampler::null(),
            pipeline: vk::Pipeline::null(),
            pipeline_layout: vk::PipelineLayout::null(),
            descriptor_set_layout: vk::DescriptorSetLayout::null(),
            descriptor_pool: vk::DescriptorPool::null(),
            descriptor_set: vk::DescriptorSet::null(),
            command_pool: vk::CommandPool::null(),
            command_buffer: vk::CommandBuffer::null(),
            initialized: false,
        }
    }
    
    /// Initialize with Vulkan device
    pub fn initialize(
        &mut self,
        device: Arc<ash::Device>,
        queue_family_index: u32,
        width: u32,
        height: u32,
    ) -> Result<(), String> {
        self.device = Some(device.clone());
        self.config.width = width;
        self.config.height = height;
        
        unsafe {
            // Create sampler
            let sampler_info = vk::SamplerCreateInfo::default()
                .mag_filter(vk::Filter::LINEAR)
                .min_filter(vk::Filter::LINEAR)
                .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
                .max_lod(1.0);
            
            self.sampler = device.create_sampler(&sampler_info, None)
                .map_err(|e| format!("Failed to create sampler: {:?}", e))?;
            
            // Create render pass for GUI compositing
            let color_attachment = vk::AttachmentDescription::default()
                .format(vk::Format::R8G8B8A8_UNORM)
                .samples(vk::SampleCountFlags::TYPE_1)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::STORE)
                .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
                .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .final_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
            
            let color_ref = vk::AttachmentReference::default()
                .attachment(0)
                .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);
            
            let subpass = vk::SubpassDescription::default()
                .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
                .color_attachments(std::slice::from_ref(&color_ref));
            
            let dependency = vk::SubpassDependency::default()
                .src_subpass(vk::SUBPASS_EXTERNAL)
                .dst_subpass(0)
                .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
                .src_access_mask(vk::AccessFlags::empty())
                .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
                .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE);
            
            let render_pass_info = vk::RenderPassCreateInfo::default()
                .attachments(std::slice::from_ref(&color_attachment))
                .subpasses(std::slice::from_ref(&subpass))
                .dependencies(std::slice::from_ref(&dependency));
            
            self.render_pass = device.create_render_pass(&render_pass_info, None)
                .map_err(|e| format!("Failed to create render pass: {:?}", e))?;
            
            // Create color image and view
            let (color_img, color_mem) = Self::create_image(&device, width, height)?;
            self.color_image = color_img;
            self.color_memory = color_mem;
            self.color_view = Self::create_image_view(&device, self.color_image)?;
            
            // Create blur image (half resolution)
            let (blur_img, blur_mem) = Self::create_image(&device, width / 2, height / 2)?;
            self.blur_image = blur_img;
            self.blur_memory = blur_mem;
            self.blur_view = Self::create_image_view(&device, self.blur_image)?;
            
            // Create framebuffers
            self.framebuffer = self.create_framebuffer(&device, self.color_view, width, height)?;
            self.blur_framebuffer = self.create_framebuffer(&device, self.blur_view, width / 2, height / 2)?;
            
            // Create command pool
            let pool_info = vk::CommandPoolCreateInfo::default()
                .queue_family_index(queue_family_index)
                .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
            
            self.command_pool = device.create_command_pool(&pool_info, None)
                .map_err(|e| format!("Failed to create command pool: {:?}", e))?;
            
            // Allocate command buffer
            let alloc_info = vk::CommandBufferAllocateInfo::default()
                .command_pool(self.command_pool)
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(1);
            
            let buffers = device.allocate_command_buffers(&alloc_info)
                .map_err(|e| format!("Failed to allocate command buffer: {:?}", e))?;
            self.command_buffer = buffers[0];
            
            // Create descriptor set layout
            let binding = vk::DescriptorSetLayoutBinding::default()
                .binding(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT);
            
            let layout_info = vk::DescriptorSetLayoutCreateInfo::default()
                .bindings(std::slice::from_ref(&binding));
            
            self.descriptor_set_layout = device.create_descriptor_set_layout(&layout_info, None)
                .map_err(|e| format!("Failed to create descriptor set layout: {:?}", e))?;
            
            // Create pipeline layout
            let push_constant = vk::PushConstantRange::default()
                .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)
                .offset(0)
                .size(64); // Transform matrix + opacity
            
            let pipeline_layout_info = vk::PipelineLayoutCreateInfo::default()
                .set_layouts(std::slice::from_ref(&self.descriptor_set_layout))
                .push_constant_ranges(std::slice::from_ref(&push_constant));
            
            self.pipeline_layout = device.create_pipeline_layout(&pipeline_layout_info, None)
                .map_err(|e| format!("Failed to create pipeline layout: {:?}", e))?;
            
            // Create descriptor pool
            let pool_size = vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(16);
            
            let pool_info = vk::DescriptorPoolCreateInfo::default()
                .pool_sizes(std::slice::from_ref(&pool_size))
                .max_sets(16);
            
            self.descriptor_pool = device.create_descriptor_pool(&pool_info, None)
                .map_err(|e| format!("Failed to create descriptor pool: {:?}", e))?;
        }
        
        self.initialized = true;
        log::info!("GUI Compositor initialized: {}x{}", width, height);
        
        Ok(())
    }
    
    fn create_image(
        device: &ash::Device,
        width: u32,
        height: u32,
    ) -> Result<(vk::Image, vk::DeviceMemory), String> {
        unsafe {
            let image_info = vk::ImageCreateInfo::default()
                .image_type(vk::ImageType::TYPE_2D)
                .extent(vk::Extent3D { width, height, depth: 1 })
                .mip_levels(1)
                .array_layers(1)
                .format(vk::Format::R8G8B8A8_UNORM)
                .tiling(vk::ImageTiling::OPTIMAL)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .usage(vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED)
                .sharing_mode(vk::SharingMode::EXCLUSIVE)
                .samples(vk::SampleCountFlags::TYPE_1);
            
            let image = device.create_image(&image_info, None)
                .map_err(|e| format!("Failed to create image: {:?}", e))?;
            
            let mem_requirements = device.get_image_memory_requirements(image);
            
            let alloc_info = vk::MemoryAllocateInfo::default()
                .allocation_size(mem_requirements.size)
                .memory_type_index(0);
            
            let memory = device.allocate_memory(&alloc_info, None)
                .map_err(|e| format!("Failed to allocate memory: {:?}", e))?;
            
            device.bind_image_memory(image, memory, 0)
                .map_err(|e| format!("Failed to bind image memory: {:?}", e))?;
            
            Ok((image, memory))
        }
    }
    
    fn create_image_view(device: &ash::Device, image: vk::Image) -> Result<vk::ImageView, String> {
        let view_info = vk::ImageViewCreateInfo::default()
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(vk::Format::R8G8B8A8_UNORM)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            });
        
        unsafe {
            device.create_image_view(&view_info, None)
                .map_err(|e| format!("Failed to create image view: {:?}", e))
        }
    }
    
    fn create_framebuffer(
        &self,
        device: &ash::Device,
        image_view: vk::ImageView,
        width: u32,
        height: u32,
    ) -> Result<vk::Framebuffer, String> {
        let framebuffer_info = vk::FramebufferCreateInfo::default()
            .render_pass(self.render_pass)
            .attachments(std::slice::from_ref(&image_view))
            .width(width)
            .height(height)
            .layers(1);
        
        unsafe {
            device.create_framebuffer(&framebuffer_info, None)
                .map_err(|e| format!("Failed to create framebuffer: {:?}", e))
        }
    }
    
    /// Add GUI element
    pub fn add_element(&mut self, element: GuiElement) -> usize {
        let index = self.elements.len();
        self.elements.push(element);
        index
    }
    
    /// Render GUI pass
    pub fn render(&mut self, queue: vk::Queue) -> Result<(), String> {
        if !self.initialized { return Ok(()); }
        
        let device = self.device.as_ref().ok_or("No device")?;
        
        unsafe {
            // Begin command buffer
            let begin_info = vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            
            device.begin_command_buffer(self.command_buffer, &begin_info)
                .map_err(|e| format!("Failed to begin command buffer: {:?}", e))?;
            
            // Begin render pass
            let clear_value = vk::ClearValue {
                color: vk::ClearColorValue { float32: [0.0, 0.0, 0.0, 0.0] },
            };
            
            let render_pass_info = vk::RenderPassBeginInfo::default()
                .render_pass(self.render_pass)
                .framebuffer(self.framebuffer)
                .render_area(vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: vk::Extent2D { width: self.config.width, height: self.config.height },
                })
                .clear_values(std::slice::from_ref(&clear_value));
            
            device.cmd_begin_render_pass(self.command_buffer, &render_pass_info, vk::SubpassContents::INLINE);
            
            // Bind pipeline and draw GUI elements
            if self.pipeline != vk::Pipeline::null() {
                device.cmd_bind_pipeline(self.command_buffer, vk::PipelineBindPoint::GRAPHICS, self.pipeline);
                
                for element in &self.elements {
                    if element.visible {
                        // Push constants for transform
                        let push_data = [
                            element.x, element.y, element.width, element.height,
                            element.opacity, 0.0, 0.0, 0.0,
                        ];
                        
                        device.cmd_push_constants(
                            self.command_buffer,
                            self.pipeline_layout,
                            vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                            0,
                            bytemuck::cast_slice(&push_data),
                        );
                        
                        // Draw quad
                        device.cmd_draw(self.command_buffer, 6, 1, 0, 0);
                    }
                }
            }
            
            device.cmd_end_render_pass(self.command_buffer);
            
            device.end_command_buffer(self.command_buffer)
                .map_err(|e| format!("Failed to end command buffer: {:?}", e))?;
            
            // Submit
            let submit_info = vk::SubmitInfo::default()
                .command_buffers(std::slice::from_ref(&self.command_buffer));
            
            device.queue_submit(queue, std::slice::from_ref(&submit_info), vk::Fence::null())
                .map_err(|e| format!("Failed to submit: {:?}", e))?;
        }
        
        Ok(())
    }
    
    /// Clear elements
    pub fn clear(&mut self) { self.elements.clear(); }
    
    /// Get element count
    pub fn element_count(&self) -> usize { self.elements.iter().filter(|e| e.visible).count() }
    
    /// Resize
    pub fn resize(&mut self, width: u32, height: u32) -> Result<(), String> {
        if width == self.config.width && height == self.config.height { return Ok(()); }
        
        let device = self.device.as_ref().ok_or("No device")?;
        
        unsafe {
            device.device_wait_idle().ok();
            
            // Destroy old resources
            if self.framebuffer != vk::Framebuffer::null() { device.destroy_framebuffer(self.framebuffer, None); }
            if self.blur_framebuffer != vk::Framebuffer::null() { device.destroy_framebuffer(self.blur_framebuffer, None); }
            if self.color_view != vk::ImageView::null() { device.destroy_image_view(self.color_view, None); }
            if self.blur_view != vk::ImageView::null() { device.destroy_image_view(self.blur_view, None); }
            if self.color_image != vk::Image::null() { device.destroy_image(self.color_image, None); }
            if self.blur_image != vk::Image::null() { device.destroy_image(self.blur_image, None); }
            if self.color_memory != vk::DeviceMemory::null() { device.free_memory(self.color_memory, None); }
            if self.blur_memory != vk::DeviceMemory::null() { device.free_memory(self.blur_memory, None); }
            
            // Create new resources
            let (color_img, color_mem) = Self::create_image(device, width, height)?;
            self.color_image = color_img;
            self.color_memory = color_mem;
            self.color_view = Self::create_image_view(device, self.color_image)?;
            let (blur_img, blur_mem) = Self::create_image(device, width / 2, height / 2)?;
            self.blur_image = blur_img;
            self.blur_memory = blur_mem;
            self.blur_view = Self::create_image_view(device, self.blur_image)?;
            self.framebuffer = self.create_framebuffer(device, self.color_view, width, height)?;
            self.blur_framebuffer = self.create_framebuffer(device, self.blur_view, width / 2, height / 2)?;
        }
        
        self.config.width = width;
        self.config.height = height;
        log::info!("Compositor resized to {}x{}", width, height);
        
        Ok(())
    }
    
    /// Shutdown
    pub fn shutdown(&mut self) {
        if let Some(device) = &self.device {
            unsafe {
                device.device_wait_idle().ok();
                
                if self.command_pool != vk::CommandPool::null() { device.destroy_command_pool(self.command_pool, None); }
                if self.descriptor_pool != vk::DescriptorPool::null() { device.destroy_descriptor_pool(self.descriptor_pool, None); }
                if self.descriptor_set_layout != vk::DescriptorSetLayout::null() { device.destroy_descriptor_set_layout(self.descriptor_set_layout, None); }
                if self.pipeline_layout != vk::PipelineLayout::null() { device.destroy_pipeline_layout(self.pipeline_layout, None); }
                if self.pipeline != vk::Pipeline::null() { device.destroy_pipeline(self.pipeline, None); }
                if self.framebuffer != vk::Framebuffer::null() { device.destroy_framebuffer(self.framebuffer, None); }
                if self.blur_framebuffer != vk::Framebuffer::null() { device.destroy_framebuffer(self.blur_framebuffer, None); }
                if self.render_pass != vk::RenderPass::null() { device.destroy_render_pass(self.render_pass, None); }
                if self.color_view != vk::ImageView::null() { device.destroy_image_view(self.color_view, None); }
                if self.blur_view != vk::ImageView::null() { device.destroy_image_view(self.blur_view, None); }
                if self.color_image != vk::Image::null() { device.destroy_image(self.color_image, None); }
                if self.blur_image != vk::Image::null() { device.destroy_image(self.blur_image, None); }
                if self.color_memory != vk::DeviceMemory::null() { device.free_memory(self.color_memory, None); }
                if self.blur_memory != vk::DeviceMemory::null() { device.free_memory(self.blur_memory, None); }
                if self.sampler != vk::Sampler::null() { device.destroy_sampler(self.sampler, None); }
            }
        }
        
        self.elements.clear();
        self.initialized = false;
        log::info!("GUI Compositor shutdown");
    }
}

impl Default for GuiCompositor { fn default() -> Self { Self::new() } }

impl Drop for GuiCompositor {
    fn drop(&mut self) { self.shutdown(); }
}
