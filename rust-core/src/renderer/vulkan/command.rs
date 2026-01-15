//! # Vulkan Command Buffers
//! 
//! Command pool and command buffer management.

use std::sync::Arc;
use ash::vk;

use super::{VulkanDevice, VulkanError};

/// Command pool wrapper
pub struct CommandPool {
    /// Device reference
    device: Arc<VulkanDevice>,
    /// Command pool handle
    pool: vk::CommandPool,
    /// Allocated command buffers
    command_buffers: Vec<vk::CommandBuffer>,
}

impl CommandPool {
    /// Create a new command pool
    pub fn new(device: Arc<VulkanDevice>) -> Result<Self, VulkanError> {
        let pool_info = vk::CommandPoolCreateInfo::default()
            .queue_family_index(device.queue_families().graphics.unwrap())
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
        
        let pool = unsafe {
            device.handle().create_command_pool(&pool_info, None)
                .map_err(|e| VulkanError::CommandBufferError(format!("Failed to create command pool: {:?}", e)))?
        };
        
        Ok(Self {
            device,
            pool,
            command_buffers: Vec::new(),
        })
    }
    
    /// Allocate command buffers
    pub fn allocate_command_buffers(&mut self, count: u32) -> Result<Vec<vk::CommandBuffer>, VulkanError> {
        let alloc_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(self.pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(count);
        
        let buffers = unsafe {
            self.device.handle().allocate_command_buffers(&alloc_info)
                .map_err(|e| VulkanError::CommandBufferError(format!("Failed to allocate command buffers: {:?}", e)))?
        };
        
        self.command_buffers.extend(&buffers);
        
        Ok(buffers)
    }
    
    /// Allocate a single command buffer
    pub fn allocate_command_buffer(&mut self) -> Result<vk::CommandBuffer, VulkanError> {
        let buffers = self.allocate_command_buffers(1)?;
        Ok(buffers[0])
    }
    
    /// Begin single-time command buffer
    pub fn begin_single_time(&mut self) -> Result<vk::CommandBuffer, VulkanError> {
        let cmd = self.allocate_command_buffer()?;
        
        let begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        
        unsafe {
            self.device.handle().begin_command_buffer(cmd, &begin_info)
                .map_err(|e| VulkanError::CommandBufferError(format!("Failed to begin command buffer: {:?}", e)))?;
        }
        
        Ok(cmd)
    }
    
    /// End and submit single-time command buffer
    pub fn end_single_time(&self, cmd: vk::CommandBuffer) -> Result<(), VulkanError> {
        unsafe {
            self.device.handle().end_command_buffer(cmd)
                .map_err(|e| VulkanError::CommandBufferError(format!("Failed to end command buffer: {:?}", e)))?;
        }
        
        let command_buffers = [cmd];
        let submit_info = vk::SubmitInfo::default()
            .command_buffers(&command_buffers);
        
        unsafe {
            self.device.handle().queue_submit(self.device.graphics_queue(), &[submit_info], vk::Fence::null())
                .map_err(|e| VulkanError::CommandBufferError(format!("Failed to submit command buffer: {:?}", e)))?;
            
            self.device.handle().queue_wait_idle(self.device.graphics_queue())
                .map_err(|e| VulkanError::CommandBufferError(format!("Failed to wait for queue: {:?}", e)))?;
            
            self.device.handle().free_command_buffers(self.pool, &command_buffers);
        }
        
        Ok(())
    }
    
    /// Reset command pool
    pub fn reset(&self) -> Result<(), VulkanError> {
        unsafe {
            self.device.handle().reset_command_pool(self.pool, vk::CommandPoolResetFlags::empty())
                .map_err(|e| VulkanError::CommandBufferError(format!("Failed to reset command pool: {:?}", e)))
        }
    }
    
    /// Get pool handle
    pub fn handle(&self) -> vk::CommandPool {
        self.pool
    }
}

impl Drop for CommandPool {
    fn drop(&mut self) {
        unsafe {
            if !self.command_buffers.is_empty() {
                self.device.handle().free_command_buffers(self.pool, &self.command_buffers);
            }
            self.device.handle().destroy_command_pool(self.pool, None);
        }
    }
}

/// Command buffer recorder helper
pub struct CommandRecorder<'a> {
    device: &'a VulkanDevice,
    cmd: vk::CommandBuffer,
}

impl<'a> CommandRecorder<'a> {
    /// Create a new command recorder
    pub fn new(device: &'a VulkanDevice, cmd: vk::CommandBuffer) -> Self {
        Self { device, cmd }
    }
    
    /// Begin recording
    pub fn begin(&self, flags: vk::CommandBufferUsageFlags) -> Result<(), VulkanError> {
        let begin_info = vk::CommandBufferBeginInfo::default()
            .flags(flags);
        
        unsafe {
            self.device.handle().begin_command_buffer(self.cmd, &begin_info)
                .map_err(|e| VulkanError::CommandBufferError(format!("Failed to begin recording: {:?}", e)))
        }
    }
    
    /// End recording
    pub fn end(&self) -> Result<(), VulkanError> {
        unsafe {
            self.device.handle().end_command_buffer(self.cmd)
                .map_err(|e| VulkanError::CommandBufferError(format!("Failed to end recording: {:?}", e)))
        }
    }
    
    /// Begin render pass
    pub fn begin_render_pass(
        &self,
        render_pass: vk::RenderPass,
        framebuffer: vk::Framebuffer,
        extent: vk::Extent2D,
        clear_values: &[vk::ClearValue],
    ) {
        let render_pass_info = vk::RenderPassBeginInfo::default()
            .render_pass(render_pass)
            .framebuffer(framebuffer)
            .render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent,
            })
            .clear_values(clear_values);
        
        unsafe {
            self.device.handle().cmd_begin_render_pass(self.cmd, &render_pass_info, vk::SubpassContents::INLINE);
        }
    }
    
    /// End render pass
    pub fn end_render_pass(&self) {
        unsafe {
            self.device.handle().cmd_end_render_pass(self.cmd);
        }
    }
    
    /// Bind graphics pipeline
    pub fn bind_pipeline(&self, pipeline: vk::Pipeline) {
        unsafe {
            self.device.handle().cmd_bind_pipeline(self.cmd, vk::PipelineBindPoint::GRAPHICS, pipeline);
        }
    }
    
    /// Bind vertex buffers
    pub fn bind_vertex_buffers(&self, first_binding: u32, buffers: &[vk::Buffer], offsets: &[vk::DeviceSize]) {
        unsafe {
            self.device.handle().cmd_bind_vertex_buffers(self.cmd, first_binding, buffers, offsets);
        }
    }
    
    /// Bind index buffer
    pub fn bind_index_buffer(&self, buffer: vk::Buffer, offset: vk::DeviceSize, index_type: vk::IndexType) {
        unsafe {
            self.device.handle().cmd_bind_index_buffer(self.cmd, buffer, offset, index_type);
        }
    }
    
    /// Bind descriptor sets
    pub fn bind_descriptor_sets(
        &self,
        layout: vk::PipelineLayout,
        first_set: u32,
        descriptor_sets: &[vk::DescriptorSet],
        dynamic_offsets: &[u32],
    ) {
        unsafe {
            self.device.handle().cmd_bind_descriptor_sets(
                self.cmd,
                vk::PipelineBindPoint::GRAPHICS,
                layout,
                first_set,
                descriptor_sets,
                dynamic_offsets,
            );
        }
    }
    
    /// Set viewport
    pub fn set_viewport(&self, viewport: vk::Viewport) {
        unsafe {
            self.device.handle().cmd_set_viewport(self.cmd, 0, &[viewport]);
        }
    }
    
    /// Set scissor
    pub fn set_scissor(&self, scissor: vk::Rect2D) {
        unsafe {
            self.device.handle().cmd_set_scissor(self.cmd, 0, &[scissor]);
        }
    }
    
    /// Draw
    pub fn draw(&self, vertex_count: u32, instance_count: u32, first_vertex: u32, first_instance: u32) {
        unsafe {
            self.device.handle().cmd_draw(self.cmd, vertex_count, instance_count, first_vertex, first_instance);
        }
    }
    
    /// Draw indexed
    pub fn draw_indexed(&self, index_count: u32, instance_count: u32, first_index: u32, vertex_offset: i32, first_instance: u32) {
        unsafe {
            self.device.handle().cmd_draw_indexed(self.cmd, index_count, instance_count, first_index, vertex_offset, first_instance);
        }
    }
    
    /// Draw indirect
    pub fn draw_indirect(&self, buffer: vk::Buffer, offset: vk::DeviceSize, draw_count: u32, stride: u32) {
        unsafe {
            self.device.handle().cmd_draw_indirect(self.cmd, buffer, offset, draw_count, stride);
        }
    }
    
    /// Draw indexed indirect
    pub fn draw_indexed_indirect(&self, buffer: vk::Buffer, offset: vk::DeviceSize, draw_count: u32, stride: u32) {
        unsafe {
            self.device.handle().cmd_draw_indexed_indirect(self.cmd, buffer, offset, draw_count, stride);
        }
    }
    
    /// Push constants
    pub fn push_constants<T: Copy>(&self, layout: vk::PipelineLayout, stages: vk::ShaderStageFlags, offset: u32, data: &T) {
        let bytes = unsafe {
            std::slice::from_raw_parts(data as *const T as *const u8, std::mem::size_of::<T>())
        };
        
        unsafe {
            self.device.handle().cmd_push_constants(self.cmd, layout, stages, offset, bytes);
        }
    }
    
    /// Pipeline barrier
    pub fn pipeline_barrier(
        &self,
        src_stage: vk::PipelineStageFlags,
        dst_stage: vk::PipelineStageFlags,
        image_barriers: &[vk::ImageMemoryBarrier],
    ) {
        unsafe {
            self.device.handle().cmd_pipeline_barrier(
                self.cmd,
                src_stage,
                dst_stage,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                image_barriers,
            );
        }
    }
    
    /// Get command buffer handle
    pub fn handle(&self) -> vk::CommandBuffer {
        self.cmd
    }
}
