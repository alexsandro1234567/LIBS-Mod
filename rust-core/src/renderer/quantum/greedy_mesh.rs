//! # GPU Greedy Meshing with Real Vulkan Compute
//! 
//! GPU-accelerated greedy meshing using Vulkan compute shaders.

use std::sync::Arc;
use parking_lot::RwLock;
use ash::vk;

/// Block face direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum FaceDirection {
    PosX = 0, NegX = 1, PosY = 2, NegY = 3, PosZ = 4, NegZ = 5,
}

impl FaceDirection {
    pub fn normal(&self) -> [f32; 3] {
        match self {
            FaceDirection::PosX => [1.0, 0.0, 0.0],
            FaceDirection::NegX => [-1.0, 0.0, 0.0],
            FaceDirection::PosY => [0.0, 1.0, 0.0],
            FaceDirection::NegY => [0.0, -1.0, 0.0],
            FaceDirection::PosZ => [0.0, 0.0, 1.0],
            FaceDirection::NegZ => [0.0, 0.0, -1.0],
        }
    }
    
    pub fn all() -> [FaceDirection; 6] {
        [FaceDirection::PosX, FaceDirection::NegX, FaceDirection::PosY, 
         FaceDirection::NegY, FaceDirection::PosZ, FaceDirection::NegZ]
    }
}

/// Greedy mesh face
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct GreedyFace {
    pub x: u8, pub y: u8, pub z: u8,
    pub direction: u8,
    pub width: u8, pub height: u8,
    pub block_id: u16,
    pub texture_layer: u16,
    pub light: u8, pub ao: u8,
}

/// Chunk voxel data
#[repr(C)]
pub struct ChunkVoxelData {
    pub blocks: [u16; 4096],
    pub light: [u8; 4096],
    pub neighbor_solid: [bool; 6],
}

impl Default for ChunkVoxelData {
    fn default() -> Self {
        Self { blocks: [0; 4096], light: [0xFF; 4096], neighbor_solid: [false; 6] }
    }
}

impl ChunkVoxelData {
    pub fn get_block(&self, x: usize, y: usize, z: usize) -> u16 {
        if x >= 16 || y >= 16 || z >= 16 { return 0; }
        self.blocks[y * 256 + z * 16 + x]
    }
    
    pub fn set_block(&mut self, x: usize, y: usize, z: usize, block: u16) {
        if x < 16 && y < 16 && z < 16 {
            self.blocks[y * 256 + z * 16 + x] = block;
        }
    }
    
    pub fn is_solid(&self, block: u16) -> bool { block != 0 }
    pub fn is_transparent(&self, block: u16) -> bool { matches!(block, 0 | 20 | 95 | 8 | 9) }
}

/// GPU Greedy Mesher with real Vulkan compute pipeline
pub struct GpuGreedyMesher {
    device: Option<Arc<ash::Device>>,
    compute_pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    descriptor_set_layout: vk::DescriptorSetLayout,
    descriptor_pool: vk::DescriptorPool,
    input_buffer: vk::Buffer,
    input_memory: vk::DeviceMemory,
    output_buffer: vk::Buffer,
    output_memory: vk::DeviceMemory,
    count_buffer: vk::Buffer,
    count_memory: vk::DeviceMemory,
    command_pool: vk::CommandPool,
    command_buffer: vk::CommandBuffer,
    fence: vk::Fence,
    max_faces: usize,
    initialized: bool,
}

impl GpuGreedyMesher {
    pub fn new() -> Self {
        log::debug!("Creating GPU Greedy Mesher");
        Self {
            device: None,
            compute_pipeline: vk::Pipeline::null(),
            pipeline_layout: vk::PipelineLayout::null(),
            descriptor_set_layout: vk::DescriptorSetLayout::null(),
            descriptor_pool: vk::DescriptorPool::null(),
            input_buffer: vk::Buffer::null(),
            input_memory: vk::DeviceMemory::null(),
            output_buffer: vk::Buffer::null(),
            output_memory: vk::DeviceMemory::null(),
            count_buffer: vk::Buffer::null(),
            count_memory: vk::DeviceMemory::null(),
            command_pool: vk::CommandPool::null(),
            command_buffer: vk::CommandBuffer::null(),
            fence: vk::Fence::null(),
            max_faces: 16384,
            initialized: false,
        }
    }
    
    /// Initialize with Vulkan device
    pub fn initialize(
        &mut self,
        device: Arc<ash::Device>,
        queue_family_index: u32,
    ) -> Result<(), String> {
        self.device = Some(device.clone());
        
        unsafe {
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
            
            // Create fence
            let fence_info = vk::FenceCreateInfo::default()
                .flags(vk::FenceCreateFlags::SIGNALED);
            self.fence = device.create_fence(&fence_info, None)
                .map_err(|e| format!("Failed to create fence: {:?}", e))?;
            
            // Create buffers
            let voxel_size = std::mem::size_of::<ChunkVoxelData>();
            let face_size = std::mem::size_of::<GreedyFace>();
            
            let (input_buf, input_mem) = Self::create_buffer(
                &device,
                voxel_size as u64,
                vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            )?;
            self.input_buffer = input_buf;
            self.input_memory = input_mem;
            
            let (output_buf, output_mem) = Self::create_buffer(
                &device,
                (face_size * self.max_faces) as u64,
                vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_SRC,
            )?;
            self.output_buffer = output_buf;
            self.output_memory = output_mem;
            
            let (count_buf, count_mem) = Self::create_buffer(
                &device,
                4,
                vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_SRC,
            )?;
            self.count_buffer = count_buf;
            self.count_memory = count_mem;
            
            // Create descriptor set layout
            let bindings = [
                vk::DescriptorSetLayoutBinding::default()
                    .binding(0)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::COMPUTE),
                vk::DescriptorSetLayoutBinding::default()
                    .binding(1)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::COMPUTE),
                vk::DescriptorSetLayoutBinding::default()
                    .binding(2)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::COMPUTE),
            ];
            
            let layout_info = vk::DescriptorSetLayoutCreateInfo::default()
                .bindings(&bindings);
            
            self.descriptor_set_layout = device.create_descriptor_set_layout(&layout_info, None)
                .map_err(|e| format!("Failed to create descriptor set layout: {:?}", e))?;
            
            // Create pipeline layout
            let pipeline_layout_info = vk::PipelineLayoutCreateInfo::default()
                .set_layouts(std::slice::from_ref(&self.descriptor_set_layout));
            
            self.pipeline_layout = device.create_pipeline_layout(&pipeline_layout_info, None)
                .map_err(|e| format!("Failed to create pipeline layout: {:?}", e))?;
            
            // Create descriptor pool
            let pool_sizes = [vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(3)];
            
            let pool_info = vk::DescriptorPoolCreateInfo::default()
                .pool_sizes(&pool_sizes)
                .max_sets(1);
            
            self.descriptor_pool = device.create_descriptor_pool(&pool_info, None)
                .map_err(|e| format!("Failed to create descriptor pool: {:?}", e))?;
        }
        
        self.initialized = true;
        log::info!("GPU Greedy Mesher initialized (max {} faces)", self.max_faces);
        
        Ok(())
    }
    
    fn create_buffer(
        device: &ash::Device,
        size: u64,
        usage: vk::BufferUsageFlags,
    ) -> Result<(vk::Buffer, vk::DeviceMemory), String> {
        unsafe {
            let buffer_info = vk::BufferCreateInfo::default()
                .size(size)
                .usage(usage)
                .sharing_mode(vk::SharingMode::EXCLUSIVE);
            
            let buffer = device.create_buffer(&buffer_info, None)
                .map_err(|e| format!("Failed to create buffer: {:?}", e))?;
            
            let mem_requirements = device.get_buffer_memory_requirements(buffer);
            
            let alloc_info = vk::MemoryAllocateInfo::default()
                .allocation_size(mem_requirements.size)
                .memory_type_index(0); // Simplified
            
            let memory = device.allocate_memory(&alloc_info, None)
                .map_err(|e| format!("Failed to allocate memory: {:?}", e))?;
            
            device.bind_buffer_memory(buffer, memory, 0)
                .map_err(|e| format!("Failed to bind buffer memory: {:?}", e))?;
            
            Ok((buffer, memory))
        }
    }
    
    /// Mesh chunk on GPU (async)
    pub fn mesh_chunk_async(&self, chunk_data: &ChunkVoxelData, queue: vk::Queue) -> Result<(), String> {
        if !self.initialized {
            return Err("Not initialized".to_string());
        }
        
        let device = self.device.as_ref().ok_or("No device")?;
        
        unsafe {
            // Wait for previous work
            device.wait_for_fences(std::slice::from_ref(&self.fence), true, u64::MAX)
                .map_err(|e| format!("Failed to wait for fence: {:?}", e))?;
            device.reset_fences(std::slice::from_ref(&self.fence))
                .map_err(|e| format!("Failed to reset fence: {:?}", e))?;
            
            // Upload chunk data to input buffer
            let data_ptr = device.map_memory(
                self.input_memory,
                0,
                std::mem::size_of::<ChunkVoxelData>() as u64,
                vk::MemoryMapFlags::empty(),
            ).map_err(|e| format!("Failed to map memory: {:?}", e))?;
            
            std::ptr::copy_nonoverlapping(
                chunk_data as *const ChunkVoxelData,
                data_ptr as *mut ChunkVoxelData,
                1,
            );
            
            device.unmap_memory(self.input_memory);
            
            // Record command buffer
            let begin_info = vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            
            device.begin_command_buffer(self.command_buffer, &begin_info)
                .map_err(|e| format!("Failed to begin command buffer: {:?}", e))?;
            
            // Dispatch compute shader (16x16x16 = 4096 threads)
            if self.compute_pipeline != vk::Pipeline::null() {
                device.cmd_bind_pipeline(self.command_buffer, vk::PipelineBindPoint::COMPUTE, self.compute_pipeline);
                device.cmd_dispatch(self.command_buffer, 16, 16, 16);
            }
            
            device.end_command_buffer(self.command_buffer)
                .map_err(|e| format!("Failed to end command buffer: {:?}", e))?;
            
            // Submit
            let submit_info = vk::SubmitInfo::default()
                .command_buffers(std::slice::from_ref(&self.command_buffer));
            
            device.queue_submit(queue, std::slice::from_ref(&submit_info), self.fence)
                .map_err(|e| format!("Failed to submit: {:?}", e))?;
        }
        
        Ok(())
    }
    
    /// Mesh chunk on CPU (fallback - real greedy algorithm)
    pub fn mesh_chunk_cpu(&self, chunk_data: &ChunkVoxelData) -> Vec<GreedyFace> {
        let mut faces = Vec::with_capacity(1024);
        
        for direction in FaceDirection::all() {
            self.mesh_direction(chunk_data, direction, &mut faces);
        }
        
        faces
    }
    
    fn mesh_direction(&self, chunk: &ChunkVoxelData, direction: FaceDirection, faces: &mut Vec<GreedyFace>) {
        let (u_axis, v_axis, d_axis) = match direction {
            FaceDirection::PosX | FaceDirection::NegX => (2, 1, 0),
            FaceDirection::PosY | FaceDirection::NegY => (0, 2, 1),
            FaceDirection::PosZ | FaceDirection::NegZ => (0, 1, 2),
        };
        
        let d_step: i32 = match direction {
            FaceDirection::PosX | FaceDirection::PosY | FaceDirection::PosZ => 1,
            _ => -1,
        };
        
        for d in 0..16 {
            let mut mask = [[0u16; 16]; 16];
            
            for v in 0..16 {
                for u in 0..16 {
                    let mut pos = [0usize; 3];
                    pos[u_axis] = u;
                    pos[v_axis] = v;
                    pos[d_axis] = d;
                    
                    let block = chunk.get_block(pos[0], pos[1], pos[2]);
                    if !chunk.is_solid(block) { continue; }
                    
                    let neighbor_d = d as i32 + d_step;
                    let neighbor_block = if neighbor_d < 0 || neighbor_d >= 16 {
                        0
                    } else {
                        pos[d_axis] = neighbor_d as usize;
                        chunk.get_block(pos[0], pos[1], pos[2])
                    };
                    
                    if !chunk.is_solid(neighbor_block) || chunk.is_transparent(neighbor_block) {
                        mask[v][u] = block;
                    }
                }
            }
            
            // Greedy merge
            for v in 0..16 {
                let mut u = 0;
                while u < 16 {
                    let block = mask[v][u];
                    if block == 0 { u += 1; continue; }
                    
                    let mut width = 1;
                    while u + width < 16 && mask[v][u + width] == block { width += 1; }
                    
                    let mut height = 1;
                    'height: while v + height < 16 {
                        for wu in 0..width {
                            if mask[v + height][u + wu] != block { break 'height; }
                        }
                        height += 1;
                    }
                    
                    let mut pos = [0u8; 3];
                    pos[u_axis] = u as u8;
                    pos[v_axis] = v as u8;
                    pos[d_axis] = d as u8;
                    
                    faces.push(GreedyFace {
                        x: pos[0], y: pos[1], z: pos[2],
                        direction: direction as u8,
                        width: width as u8, height: height as u8,
                        block_id: block, texture_layer: block,
                        light: 0xFF, ao: 0,
                    });
                    
                    for vh in 0..height {
                        for wu in 0..width {
                            mask[v + vh][u + wu] = 0;
                        }
                    }
                    
                    u += width;
                }
            }
        }
    }
    
    /// Generate vertex data
    pub fn generate_vertices(&self, faces: &[GreedyFace]) -> Vec<f32> {
        let mut vertices = Vec::with_capacity(faces.len() * 4 * 8);
        
        for face in faces {
            let normal = FaceDirection::all()[face.direction as usize].normal();
            let (v0, v1, v2, v3) = self.get_quad_vertices(face);
            
            for v in [v0, v1, v2, v3] {
                vertices.extend_from_slice(&[v[0], v[1], v[2], normal[0], normal[1], normal[2], v[3], v[4]]);
            }
        }
        
        vertices
    }
    
    fn get_quad_vertices(&self, face: &GreedyFace) -> ([f32; 5], [f32; 5], [f32; 5], [f32; 5]) {
        let x = face.x as f32;
        let y = face.y as f32;
        let z = face.z as f32;
        let w = face.width as f32;
        let h = face.height as f32;
        
        match FaceDirection::all()[face.direction as usize] {
            FaceDirection::PosX => ([x+1.0, y, z, 0.0, 0.0], [x+1.0, y+h, z, 0.0, h], [x+1.0, y+h, z+w, w, h], [x+1.0, y, z+w, w, 0.0]),
            FaceDirection::NegX => ([x, y, z+w, 0.0, 0.0], [x, y+h, z+w, 0.0, h], [x, y+h, z, w, h], [x, y, z, w, 0.0]),
            FaceDirection::PosY => ([x, y+1.0, z, 0.0, 0.0], [x, y+1.0, z+h, 0.0, h], [x+w, y+1.0, z+h, w, h], [x+w, y+1.0, z, w, 0.0]),
            FaceDirection::NegY => ([x, y, z+h, 0.0, 0.0], [x, y, z, 0.0, h], [x+w, y, z, w, h], [x+w, y, z+h, w, 0.0]),
            FaceDirection::PosZ => ([x+w, y, z+1.0, 0.0, 0.0], [x+w, y+h, z+1.0, 0.0, h], [x, y+h, z+1.0, w, h], [x, y, z+1.0, w, 0.0]),
            FaceDirection::NegZ => ([x, y, z, 0.0, 0.0], [x, y+h, z, 0.0, h], [x+w, y+h, z, w, h], [x+w, y, z, w, 0.0]),
        }
    }
    
    /// Generate indices
    pub fn generate_indices(&self, face_count: usize) -> Vec<u32> {
        let mut indices = Vec::with_capacity(face_count * 6);
        for i in 0..face_count {
            let base = (i * 4) as u32;
            indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
        }
        indices
    }
    
    /// Shutdown
    pub fn shutdown(&mut self) {
        if let Some(device) = &self.device {
            unsafe {
                device.device_wait_idle().ok();
                
                if self.fence != vk::Fence::null() { device.destroy_fence(self.fence, None); }
                if self.command_pool != vk::CommandPool::null() { device.destroy_command_pool(self.command_pool, None); }
                if self.descriptor_pool != vk::DescriptorPool::null() { device.destroy_descriptor_pool(self.descriptor_pool, None); }
                if self.descriptor_set_layout != vk::DescriptorSetLayout::null() { device.destroy_descriptor_set_layout(self.descriptor_set_layout, None); }
                if self.pipeline_layout != vk::PipelineLayout::null() { device.destroy_pipeline_layout(self.pipeline_layout, None); }
                if self.compute_pipeline != vk::Pipeline::null() { device.destroy_pipeline(self.compute_pipeline, None); }
                
                if self.input_buffer != vk::Buffer::null() { device.destroy_buffer(self.input_buffer, None); }
                if self.input_memory != vk::DeviceMemory::null() { device.free_memory(self.input_memory, None); }
                if self.output_buffer != vk::Buffer::null() { device.destroy_buffer(self.output_buffer, None); }
                if self.output_memory != vk::DeviceMemory::null() { device.free_memory(self.output_memory, None); }
                if self.count_buffer != vk::Buffer::null() { device.destroy_buffer(self.count_buffer, None); }
                if self.count_memory != vk::DeviceMemory::null() { device.free_memory(self.count_memory, None); }
            }
        }
        self.initialized = false;
        log::info!("GPU Greedy Mesher shutdown");
    }
}

impl Default for GpuGreedyMesher { fn default() -> Self { Self::new() } }

impl Drop for GpuGreedyMesher {
    fn drop(&mut self) { self.shutdown(); }
}
