//! # Vulkan Pipeline
//! 
//! Graphics and compute pipeline management.

use std::sync::Arc;
use ash::vk;

use super::{VulkanConfig, VulkanDevice, VulkanError, Swapchain};

/// Graphics pipeline wrapper
pub struct Pipeline {
    /// Device reference
    device: Arc<VulkanDevice>,
    /// Pipeline layout
    layout: vk::PipelineLayout,
    /// Graphics pipeline
    pipeline: vk::Pipeline,
    /// Render pass
    render_pass: vk::RenderPass,
    /// Descriptor set layout
    descriptor_set_layout: vk::DescriptorSetLayout,
    /// Is mesh shader pipeline
    is_mesh_shader: bool,
}

impl Pipeline {
    /// Create a new graphics pipeline
    pub fn new(
        device: Arc<VulkanDevice>,
        swapchain: &Swapchain,
        config: &VulkanConfig,
    ) -> Result<Self, VulkanError> {
        // Create render pass
        let render_pass = Self::create_render_pass(&device, swapchain)?;
        
        // Create descriptor set layout
        let descriptor_set_layout = Self::create_descriptor_set_layout(&device)?;
        
        // Create pipeline layout
        let layouts = [descriptor_set_layout];
        let push_constant_ranges = [
            vk::PushConstantRange::default()
                .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)
                .offset(0)
                .size(128), // 128 bytes for push constants
        ];
        
        let layout_info = vk::PipelineLayoutCreateInfo::default()
            .set_layouts(&layouts)
            .push_constant_ranges(&push_constant_ranges);
        
        let layout = unsafe {
            device.handle().create_pipeline_layout(&layout_info, None)
                .map_err(|e| VulkanError::PipelineCreationFailed(format!("Failed to create pipeline layout: {:?}", e)))?
        };
        
        // Determine if using mesh shaders
        let is_mesh_shader = device.supports_mesh_shaders() && config.mesh_shaders_enabled;
        
        // Create pipeline (placeholder - would load actual shaders)
        let pipeline = if is_mesh_shader {
            Self::create_mesh_shader_pipeline(&device, layout, render_pass, swapchain)?
        } else {
            Self::create_vertex_pipeline(&device, layout, render_pass, swapchain)?
        };
        
        Ok(Self {
            device,
            layout,
            pipeline,
            render_pass,
            descriptor_set_layout,
            is_mesh_shader,
        })
    }
    
    /// Create render pass
    fn create_render_pass(device: &VulkanDevice, swapchain: &Swapchain) -> Result<vk::RenderPass, VulkanError> {
        let color_attachment = vk::AttachmentDescription::default()
            .format(swapchain.format())
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::PRESENT_SRC_KHR);
        
        let depth_attachment = vk::AttachmentDescription::default()
            .format(swapchain.depth_format())
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::DONT_CARE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);
        
        let attachments = [color_attachment, depth_attachment];
        
        let color_attachment_ref = vk::AttachmentReference::default()
            .attachment(0)
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);
        
        let depth_attachment_ref = vk::AttachmentReference::default()
            .attachment(1)
            .layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);
        
        let color_attachments = [color_attachment_ref];
        
        let subpass = vk::SubpassDescription::default()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(&color_attachments)
            .depth_stencil_attachment(&depth_attachment_ref);
        
        let subpasses = [subpass];
        
        let dependency = vk::SubpassDependency::default()
            .src_subpass(vk::SUBPASS_EXTERNAL)
            .dst_subpass(0)
            .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS)
            .src_access_mask(vk::AccessFlags::empty())
            .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS)
            .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE);
        
        let dependencies = [dependency];
        
        let render_pass_info = vk::RenderPassCreateInfo::default()
            .attachments(&attachments)
            .subpasses(&subpasses)
            .dependencies(&dependencies);
        
        unsafe {
            device.handle().create_render_pass(&render_pass_info, None)
                .map_err(|e| VulkanError::PipelineCreationFailed(format!("Failed to create render pass: {:?}", e)))
        }
    }
    
    /// Create descriptor set layout
    fn create_descriptor_set_layout(device: &VulkanDevice) -> Result<vk::DescriptorSetLayout, VulkanError> {
        let bindings = [
            // Uniform buffer for matrices
            vk::DescriptorSetLayoutBinding::default()
                .binding(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT),
            // Combined image sampler for textures
            vk::DescriptorSetLayoutBinding::default()
                .binding(1)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(16) // Array of textures
                .stage_flags(vk::ShaderStageFlags::FRAGMENT),
            // Storage buffer for instance data
            vk::DescriptorSetLayoutBinding::default()
                .binding(2)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::VERTEX),
        ];
        
        let layout_info = vk::DescriptorSetLayoutCreateInfo::default()
            .bindings(&bindings);
        
        unsafe {
            device.handle().create_descriptor_set_layout(&layout_info, None)
                .map_err(|e| VulkanError::PipelineCreationFailed(format!("Failed to create descriptor set layout: {:?}", e)))
        }
    }
    
    /// Create traditional vertex shader pipeline
    fn create_vertex_pipeline(
        device: &VulkanDevice,
        layout: vk::PipelineLayout,
        render_pass: vk::RenderPass,
        swapchain: &Swapchain,
    ) -> Result<vk::Pipeline, VulkanError> {
        // In production, would load compiled SPIR-V shaders
        // For now, create a minimal pipeline configuration
        
        // Vertex input state
        let vertex_binding = vk::VertexInputBindingDescription::default()
            .binding(0)
            .stride(32) // Position (12) + Normal (12) + UV (8)
            .input_rate(vk::VertexInputRate::VERTEX);
        
        let vertex_attributes = [
            vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(0)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(0),
            vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(1)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(12),
            vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(2)
                .format(vk::Format::R32G32_SFLOAT)
                .offset(24),
        ];
        
        let bindings = [vertex_binding];
        
        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_binding_descriptions(&bindings)
            .vertex_attribute_descriptions(&vertex_attributes);
        
        let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);
        
        let viewport = vk::Viewport::default()
            .x(0.0)
            .y(0.0)
            .width(swapchain.extent().width as f32)
            .height(swapchain.extent().height as f32)
            .min_depth(0.0)
            .max_depth(1.0);
        
        let scissor = vk::Rect2D::default()
            .offset(vk::Offset2D { x: 0, y: 0 })
            .extent(swapchain.extent());
        
        let viewports = [viewport];
        let scissors = [scissor];
        
        let viewport_state = vk::PipelineViewportStateCreateInfo::default()
            .viewports(&viewports)
            .scissors(&scissors);
        
        let rasterizer = vk::PipelineRasterizationStateCreateInfo::default()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(vk::CullModeFlags::BACK)
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .depth_bias_enable(false);
        
        let multisampling = vk::PipelineMultisampleStateCreateInfo::default()
            .sample_shading_enable(false)
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);
        
        let depth_stencil = vk::PipelineDepthStencilStateCreateInfo::default()
            .depth_test_enable(true)
            .depth_write_enable(true)
            .depth_compare_op(vk::CompareOp::LESS)
            .depth_bounds_test_enable(false)
            .stencil_test_enable(false);
        
        let color_blend_attachment = vk::PipelineColorBlendAttachmentState::default()
            .color_write_mask(vk::ColorComponentFlags::RGBA)
            .blend_enable(false);
        
        let color_blend_attachments = [color_blend_attachment];
        
        let color_blending = vk::PipelineColorBlendStateCreateInfo::default()
            .logic_op_enable(false)
            .attachments(&color_blend_attachments);
        
        let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        
        let dynamic_state = vk::PipelineDynamicStateCreateInfo::default()
            .dynamic_states(&dynamic_states);
        
        // Note: In production, would have actual shader modules
        // This is a placeholder that won't actually work without shaders
        let pipeline_info = vk::GraphicsPipelineCreateInfo::default()
            .vertex_input_state(&vertex_input_info)
            .input_assembly_state(&input_assembly)
            .viewport_state(&viewport_state)
            .rasterization_state(&rasterizer)
            .multisample_state(&multisampling)
            .depth_stencil_state(&depth_stencil)
            .color_blend_state(&color_blending)
            .dynamic_state(&dynamic_state)
            .layout(layout)
            .render_pass(render_pass)
            .subpass(0);
        
        // Return null pipeline for now (would need actual shaders)
        Ok(vk::Pipeline::null())
    }
    
    /// Create mesh shader pipeline
    fn create_mesh_shader_pipeline(
        device: &VulkanDevice,
        layout: vk::PipelineLayout,
        render_pass: vk::RenderPass,
        swapchain: &Swapchain,
    ) -> Result<vk::Pipeline, VulkanError> {
        // Mesh shader pipeline configuration
        // Would load task and mesh shaders instead of vertex shader
        
        // For now, return null pipeline (would need actual shaders)
        Ok(vk::Pipeline::null())
    }
    
    /// Get pipeline handle
    pub fn handle(&self) -> vk::Pipeline {
        self.pipeline
    }
    
    /// Get pipeline layout
    pub fn layout(&self) -> vk::PipelineLayout {
        self.layout
    }
    
    /// Get render pass
    pub fn render_pass(&self) -> vk::RenderPass {
        self.render_pass
    }
    
    /// Get descriptor set layout
    pub fn descriptor_set_layout(&self) -> vk::DescriptorSetLayout {
        self.descriptor_set_layout
    }
    
    /// Check if using mesh shaders
    pub fn is_mesh_shader(&self) -> bool {
        self.is_mesh_shader
    }
}

impl Drop for Pipeline {
    fn drop(&mut self) {
        unsafe {
            if self.pipeline != vk::Pipeline::null() {
                self.device.handle().destroy_pipeline(self.pipeline, None);
            }
            self.device.handle().destroy_pipeline_layout(self.layout, None);
            self.device.handle().destroy_render_pass(self.render_pass, None);
            self.device.handle().destroy_descriptor_set_layout(self.descriptor_set_layout, None);
        }
    }
}
