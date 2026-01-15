//! # Mesh Shader Module
//! 
//! Mesh shader implementation for efficient chunk rendering.
//! Uses VK_EXT_mesh_shader for modern GPU-driven rendering.

use std::sync::Arc;
use ash::vk;

use super::{VulkanDevice, VulkanError, Buffer, BufferType};

/// Maximum meshlets per chunk
pub const MAX_MESHLETS_PER_CHUNK: usize = 4096;

/// Maximum vertices per meshlet
pub const MAX_VERTICES_PER_MESHLET: usize = 64;

/// Maximum primitives (triangles) per meshlet
pub const MAX_PRIMITIVES_PER_MESHLET: usize = 124;

/// Meshlet data structure (GPU-side)
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Meshlet {
    /// Offset into vertex buffer
    pub vertex_offset: u32,
    /// Number of vertices in this meshlet
    pub vertex_count: u32,
    /// Offset into primitive index buffer
    pub primitive_offset: u32,
    /// Number of primitives (triangles)
    pub primitive_count: u32,
    /// Bounding sphere center (xyz) and radius (w)
    pub bounding_sphere: [f32; 4],
    /// Cone axis (xyz) and cutoff (w) for backface culling
    pub cone_axis_cutoff: [f32; 4],
}

/// Chunk mesh data for mesh shader rendering
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct ChunkMeshData {
    /// Chunk position in world space
    pub chunk_pos: [f32; 4],
    /// Number of meshlets in this chunk
    pub meshlet_count: u32,
    /// Offset into global meshlet buffer
    pub meshlet_offset: u32,
    /// LOD level (0 = highest detail)
    pub lod_level: u32,
    /// Flags (visibility, etc.)
    pub flags: u32,
}

/// Vertex data for mesh shader
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct MeshVertex {
    /// Position (xyz) and packed normal (w)
    pub position_normal: [f32; 4],
    /// UV coordinates and block ID
    pub uv_block: [f32; 4],
    /// Ambient occlusion and light level
    pub ao_light: [f32; 4],
}

/// Mesh shader pipeline for chunk rendering
pub struct MeshShaderPipeline {
    /// Device reference
    device: Arc<VulkanDevice>,
    /// Pipeline layout
    layout: vk::PipelineLayout,
    /// Mesh shader pipeline
    pipeline: vk::Pipeline,
    /// Descriptor set layout
    descriptor_layout: vk::DescriptorSetLayout,
    /// Descriptor pool
    descriptor_pool: vk::DescriptorPool,
    /// Meshlet buffer (all chunks)
    meshlet_buffer: Option<Buffer>,
    /// Vertex buffer (all chunks)
    vertex_buffer: Option<Buffer>,
    /// Primitive index buffer
    primitive_buffer: Option<Buffer>,
    /// Chunk data buffer
    chunk_buffer: Option<Buffer>,
    /// Draw indirect buffer
    indirect_buffer: Option<Buffer>,
    /// Maximum chunks
    max_chunks: usize,
    /// Current chunk count
    chunk_count: usize,
}

impl MeshShaderPipeline {
    /// Create a new mesh shader pipeline
    pub fn new(
        device: Arc<VulkanDevice>,
        render_pass: vk::RenderPass,
        max_chunks: usize,
    ) -> Result<Self, VulkanError> {
        if !device.supports_mesh_shaders() {
            return Err(VulkanError::PipelineCreationFailed("Mesh shaders not supported".to_string()));
        }
        
        // Create descriptor set layout
        let descriptor_layout = Self::create_descriptor_layout(&device)?;
        
        // Create pipeline layout
        let layouts = [descriptor_layout];
        let push_constant_range = vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::MESH_EXT | vk::ShaderStageFlags::FRAGMENT)
            .offset(0)
            .size(128);
        
        let push_ranges = [push_constant_range];
        
        let layout_info = vk::PipelineLayoutCreateInfo::default()
            .set_layouts(&layouts)
            .push_constant_ranges(&push_ranges);
        
        let layout = unsafe {
            device.handle().create_pipeline_layout(&layout_info, None)
                .map_err(|e| VulkanError::PipelineCreationFailed(format!("Failed to create pipeline layout: {:?}", e)))?
        };
        
        // Create descriptor pool
        let pool_sizes = [
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(10),
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(16),
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(4),
        ];
        
        let pool_info = vk::DescriptorPoolCreateInfo::default()
            .max_sets(4)
            .pool_sizes(&pool_sizes);
        
        let descriptor_pool = unsafe {
            device.handle().create_descriptor_pool(&pool_info, None)
                .map_err(|e| VulkanError::PipelineCreationFailed(format!("Failed to create descriptor pool: {:?}", e)))?
        };
        
        // Create pipeline (would load actual shaders)
        let pipeline = Self::create_pipeline(&device, layout, render_pass)?;
        
        // Allocate GPU buffers
        let meshlet_buffer = Some(Buffer::new(
            device.clone(),
            (max_chunks * MAX_MESHLETS_PER_CHUNK * std::mem::size_of::<Meshlet>()) as u64,
            BufferType::Storage,
        )?);
        
        let vertex_buffer = Some(Buffer::new(
            device.clone(),
            (max_chunks * MAX_MESHLETS_PER_CHUNK * MAX_VERTICES_PER_MESHLET * std::mem::size_of::<MeshVertex>()) as u64,
            BufferType::Storage,
        )?);
        
        let primitive_buffer = Some(Buffer::new(
            device.clone(),
            (max_chunks * MAX_MESHLETS_PER_CHUNK * MAX_PRIMITIVES_PER_MESHLET * 3) as u64,
            BufferType::Storage,
        )?);
        
        let chunk_buffer = Some(Buffer::new(
            device.clone(),
            (max_chunks * std::mem::size_of::<ChunkMeshData>()) as u64,
            BufferType::Storage,
        )?);
        
        let indirect_buffer = Some(Buffer::new(
            device.clone(),
            (max_chunks * std::mem::size_of::<vk::DrawMeshTasksIndirectCommandEXT>()) as u64,
            BufferType::Storage,
        )?);
        
        Ok(Self {
            device,
            layout,
            pipeline,
            descriptor_layout,
            descriptor_pool,
            meshlet_buffer,
            vertex_buffer,
            primitive_buffer,
            chunk_buffer,
            indirect_buffer,
            max_chunks,
            chunk_count: 0,
        })
    }
    
    /// Create descriptor set layout
    fn create_descriptor_layout(device: &VulkanDevice) -> Result<vk::DescriptorSetLayout, VulkanError> {
        let bindings = [
            // Binding 0: Camera/view uniform buffer
            vk::DescriptorSetLayoutBinding::default()
                .binding(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::MESH_EXT | vk::ShaderStageFlags::TASK_EXT),
            // Binding 1: Meshlet buffer
            vk::DescriptorSetLayoutBinding::default()
                .binding(1)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::MESH_EXT),
            // Binding 2: Vertex buffer
            vk::DescriptorSetLayoutBinding::default()
                .binding(2)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::MESH_EXT),
            // Binding 3: Primitive index buffer
            vk::DescriptorSetLayoutBinding::default()
                .binding(3)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::MESH_EXT),
            // Binding 4: Chunk data buffer
            vk::DescriptorSetLayoutBinding::default()
                .binding(4)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::TASK_EXT | vk::ShaderStageFlags::MESH_EXT),
            // Binding 5: Block texture array
            vk::DescriptorSetLayoutBinding::default()
                .binding(5)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT),
        ];
        
        let layout_info = vk::DescriptorSetLayoutCreateInfo::default()
            .bindings(&bindings);
        
        unsafe {
            device.handle().create_descriptor_set_layout(&layout_info, None)
                .map_err(|e| VulkanError::PipelineCreationFailed(format!("Failed to create descriptor layout: {:?}", e)))
        }
    }
    
    /// Create mesh shader pipeline
    fn create_pipeline(
        device: &VulkanDevice,
        layout: vk::PipelineLayout,
        render_pass: vk::RenderPass,
    ) -> Result<vk::Pipeline, VulkanError> {
        // In production, would load compiled SPIR-V shaders
        // For now, return null pipeline
        Ok(vk::Pipeline::null())
    }
    
    /// Upload chunk mesh data
    pub fn upload_chunk(
        &mut self,
        chunk_index: usize,
        meshlets: &[Meshlet],
        vertices: &[MeshVertex],
        primitives: &[u8],
        chunk_data: &ChunkMeshData,
    ) -> Result<(), VulkanError> {
        if chunk_index >= self.max_chunks {
            return Err(VulkanError::BufferCreationFailed("Chunk index out of range".to_string()));
        }
        
        // Upload meshlets
        if let Some(ref buffer) = self.meshlet_buffer {
            let offset = chunk_index * MAX_MESHLETS_PER_CHUNK * std::mem::size_of::<Meshlet>();
            // Would use staging buffer for device-local memory
        }
        
        // Upload vertices
        if let Some(ref buffer) = self.vertex_buffer {
            let offset = chunk_index * MAX_MESHLETS_PER_CHUNK * MAX_VERTICES_PER_MESHLET * std::mem::size_of::<MeshVertex>();
            // Would use staging buffer
        }
        
        // Upload primitives
        if let Some(ref buffer) = self.primitive_buffer {
            let offset = chunk_index * MAX_MESHLETS_PER_CHUNK * MAX_PRIMITIVES_PER_MESHLET * 3;
            // Would use staging buffer
        }
        
        // Upload chunk data
        if let Some(ref buffer) = self.chunk_buffer {
            let offset = chunk_index * std::mem::size_of::<ChunkMeshData>();
            // Would use staging buffer
        }
        
        if chunk_index >= self.chunk_count {
            self.chunk_count = chunk_index + 1;
        }
        
        Ok(())
    }
    
    /// Build meshlets from raw chunk vertex data
    pub fn build_meshlets(
        vertices: &[MeshVertex],
        indices: &[u32],
    ) -> (Vec<Meshlet>, Vec<MeshVertex>, Vec<u8>) {
        let mut meshlets = Vec::new();
        let mut out_vertices = Vec::new();
        let mut out_primitives = Vec::new();
        
        // Simple meshlet builder
        let mut current_meshlet = Meshlet::default();
        let mut meshlet_vertices: Vec<MeshVertex> = Vec::new();
        let mut meshlet_indices: Vec<u8> = Vec::new();
        let mut vertex_map: std::collections::HashMap<u32, u8> = std::collections::HashMap::new();
        
        for triangle in indices.chunks(3) {
            if triangle.len() != 3 {
                continue;
            }
            
            // Check if we need to start a new meshlet
            let new_vertices = triangle.iter()
                .filter(|&&idx| !vertex_map.contains_key(&idx))
                .count();
            
            if meshlet_vertices.len() + new_vertices > MAX_VERTICES_PER_MESHLET
                || meshlet_indices.len() / 3 >= MAX_PRIMITIVES_PER_MESHLET
            {
                // Finalize current meshlet
                if !meshlet_vertices.is_empty() {
                    current_meshlet.vertex_offset = out_vertices.len() as u32;
                    current_meshlet.vertex_count = meshlet_vertices.len() as u32;
                    current_meshlet.primitive_offset = out_primitives.len() as u32;
                    current_meshlet.primitive_count = (meshlet_indices.len() / 3) as u32;
                    
                    // Calculate bounding sphere
                    current_meshlet.bounding_sphere = Self::calculate_bounding_sphere(&meshlet_vertices);
                    
                    meshlets.push(current_meshlet);
                    out_vertices.extend(&meshlet_vertices);
                    out_primitives.extend(&meshlet_indices);
                }
                
                // Start new meshlet
                current_meshlet = Meshlet::default();
                meshlet_vertices.clear();
                meshlet_indices.clear();
                vertex_map.clear();
            }
            
            // Add triangle to meshlet
            for &idx in triangle {
                let local_idx = *vertex_map.entry(idx).or_insert_with(|| {
                    let new_idx = meshlet_vertices.len() as u8;
                    meshlet_vertices.push(vertices[idx as usize]);
                    new_idx
                });
                meshlet_indices.push(local_idx);
            }
        }
        
        // Finalize last meshlet
        if !meshlet_vertices.is_empty() {
            current_meshlet.vertex_offset = out_vertices.len() as u32;
            current_meshlet.vertex_count = meshlet_vertices.len() as u32;
            current_meshlet.primitive_offset = out_primitives.len() as u32;
            current_meshlet.primitive_count = (meshlet_indices.len() / 3) as u32;
            current_meshlet.bounding_sphere = Self::calculate_bounding_sphere(&meshlet_vertices);
            
            meshlets.push(current_meshlet);
            out_vertices.extend(&meshlet_vertices);
            out_primitives.extend(&meshlet_indices);
        }
        
        (meshlets, out_vertices, out_primitives)
    }
    
    /// Calculate bounding sphere for a set of vertices
    fn calculate_bounding_sphere(vertices: &[MeshVertex]) -> [f32; 4] {
        if vertices.is_empty() {
            return [0.0, 0.0, 0.0, 0.0];
        }
        
        // Calculate center
        let mut center = [0.0f32; 3];
        for v in vertices {
            center[0] += v.position_normal[0];
            center[1] += v.position_normal[1];
            center[2] += v.position_normal[2];
        }
        let n = vertices.len() as f32;
        center[0] /= n;
        center[1] /= n;
        center[2] /= n;
        
        // Calculate radius
        let mut max_dist_sq = 0.0f32;
        for v in vertices {
            let dx = v.position_normal[0] - center[0];
            let dy = v.position_normal[1] - center[1];
            let dz = v.position_normal[2] - center[2];
            let dist_sq = dx * dx + dy * dy + dz * dz;
            max_dist_sq = max_dist_sq.max(dist_sq);
        }
        
        [center[0], center[1], center[2], max_dist_sq.sqrt()]
    }
    
    /// Record draw commands
    pub fn record_draw(
        &self,
        cmd: vk::CommandBuffer,
        descriptor_set: vk::DescriptorSet,
    ) {
        if self.pipeline == vk::Pipeline::null() || self.chunk_count == 0 {
            return;
        }
        
        unsafe {
            // Bind pipeline
            self.device.handle().cmd_bind_pipeline(
                cmd,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline,
            );
            
            // Bind descriptor set
            self.device.handle().cmd_bind_descriptor_sets(
                cmd,
                vk::PipelineBindPoint::GRAPHICS,
                self.layout,
                0,
                &[descriptor_set],
                &[],
            );
            
            // Draw mesh tasks
            // Would use vkCmdDrawMeshTasksIndirectEXT for indirect drawing
        }
    }
    
    /// Get pipeline handle
    pub fn pipeline(&self) -> vk::Pipeline {
        self.pipeline
    }
    
    /// Get pipeline layout
    pub fn layout(&self) -> vk::PipelineLayout {
        self.layout
    }
    
    /// Get descriptor layout
    pub fn descriptor_layout(&self) -> vk::DescriptorSetLayout {
        self.descriptor_layout
    }
    
    /// Get chunk count
    pub fn chunk_count(&self) -> usize {
        self.chunk_count
    }
}

impl Drop for MeshShaderPipeline {
    fn drop(&mut self) {
        unsafe {
            if self.pipeline != vk::Pipeline::null() {
                self.device.handle().destroy_pipeline(self.pipeline, None);
            }
            self.device.handle().destroy_pipeline_layout(self.layout, None);
            self.device.handle().destroy_descriptor_pool(self.descriptor_pool, None);
            self.device.handle().destroy_descriptor_set_layout(self.descriptor_layout, None);
        }
    }
}

/// Chunk mesher for converting block data to mesh shader format
pub struct ChunkMesher {
    /// Block face visibility cache
    visibility_cache: Vec<u8>,
}

impl ChunkMesher {
    /// Create a new chunk mesher
    pub fn new() -> Self {
        Self {
            visibility_cache: Vec::with_capacity(16 * 16 * 384), // Max chunk size
        }
    }
    
    /// Mesh a chunk section (16x16x16)
    pub fn mesh_section(
        &mut self,
        blocks: &[u16; 4096],
        section_y: i32,
        neighbors: &ChunkNeighbors,
    ) -> (Vec<MeshVertex>, Vec<u32>) {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        
        for y in 0..16 {
            for z in 0..16 {
                for x in 0..16 {
                    let idx = (y << 8) | (z << 4) | x;
                    let block = blocks[idx];
                    
                    if block == 0 {
                        continue; // Air
                    }
                    
                    // Check each face
                    let world_y = section_y * 16 + y as i32;
                    
                    // +X face
                    if x == 15 || blocks[idx + 1] == 0 {
                        self.add_face(&mut vertices, &mut indices, x, y, z, block, Face::PosX);
                    }
                    
                    // -X face
                    if x == 0 || blocks[idx - 1] == 0 {
                        self.add_face(&mut vertices, &mut indices, x, y, z, block, Face::NegX);
                    }
                    
                    // +Y face
                    if y == 15 || blocks[idx + 256] == 0 {
                        self.add_face(&mut vertices, &mut indices, x, y, z, block, Face::PosY);
                    }
                    
                    // -Y face
                    if y == 0 || blocks[idx - 256] == 0 {
                        self.add_face(&mut vertices, &mut indices, x, y, z, block, Face::NegY);
                    }
                    
                    // +Z face
                    if z == 15 || blocks[idx + 16] == 0 {
                        self.add_face(&mut vertices, &mut indices, x, y, z, block, Face::PosZ);
                    }
                    
                    // -Z face
                    if z == 0 || blocks[idx - 16] == 0 {
                        self.add_face(&mut vertices, &mut indices, x, y, z, block, Face::NegZ);
                    }
                }
            }
        }
        
        (vertices, indices)
    }
    
    /// Add a face to the mesh
    fn add_face(
        &self,
        vertices: &mut Vec<MeshVertex>,
        indices: &mut Vec<u32>,
        x: usize,
        y: usize,
        z: usize,
        block: u16,
        face: Face,
    ) {
        let base_idx = vertices.len() as u32;
        
        let (positions, normal, uvs) = face.get_geometry(x as f32, y as f32, z as f32);
        
        for i in 0..4 {
            vertices.push(MeshVertex {
                position_normal: [positions[i][0], positions[i][1], positions[i][2], Self::pack_normal(normal)],
                uv_block: [uvs[i][0], uvs[i][1], block as f32, 0.0],
                ao_light: [1.0, 1.0, 1.0, 1.0], // Would calculate AO
            });
        }
        
        // Two triangles per face
        indices.extend_from_slice(&[
            base_idx, base_idx + 1, base_idx + 2,
            base_idx, base_idx + 2, base_idx + 3,
        ]);
    }
    
    /// Pack normal into a single float
    fn pack_normal(normal: [f32; 3]) -> f32 {
        let x = ((normal[0] * 0.5 + 0.5) * 255.0) as u32;
        let y = ((normal[1] * 0.5 + 0.5) * 255.0) as u32;
        let z = ((normal[2] * 0.5 + 0.5) * 255.0) as u32;
        f32::from_bits((x << 16) | (y << 8) | z)
    }
}

impl Default for ChunkMesher {
    fn default() -> Self {
        Self::new()
    }
}

/// Face direction
#[derive(Debug, Clone, Copy)]
enum Face {
    PosX,
    NegX,
    PosY,
    NegY,
    PosZ,
    NegZ,
}

impl Face {
    /// Get face geometry
    fn get_geometry(&self, x: f32, y: f32, z: f32) -> ([[f32; 3]; 4], [f32; 3], [[f32; 2]; 4]) {
        match self {
            Face::PosX => (
                [
                    [x + 1.0, y, z],
                    [x + 1.0, y + 1.0, z],
                    [x + 1.0, y + 1.0, z + 1.0],
                    [x + 1.0, y, z + 1.0],
                ],
                [1.0, 0.0, 0.0],
                [[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]],
            ),
            Face::NegX => (
                [
                    [x, y, z + 1.0],
                    [x, y + 1.0, z + 1.0],
                    [x, y + 1.0, z],
                    [x, y, z],
                ],
                [-1.0, 0.0, 0.0],
                [[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]],
            ),
            Face::PosY => (
                [
                    [x, y + 1.0, z],
                    [x, y + 1.0, z + 1.0],
                    [x + 1.0, y + 1.0, z + 1.0],
                    [x + 1.0, y + 1.0, z],
                ],
                [0.0, 1.0, 0.0],
                [[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]],
            ),
            Face::NegY => (
                [
                    [x, y, z + 1.0],
                    [x, y, z],
                    [x + 1.0, y, z],
                    [x + 1.0, y, z + 1.0],
                ],
                [0.0, -1.0, 0.0],
                [[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]],
            ),
            Face::PosZ => (
                [
                    [x + 1.0, y, z + 1.0],
                    [x + 1.0, y + 1.0, z + 1.0],
                    [x, y + 1.0, z + 1.0],
                    [x, y, z + 1.0],
                ],
                [0.0, 0.0, 1.0],
                [[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]],
            ),
            Face::NegZ => (
                [
                    [x, y, z],
                    [x, y + 1.0, z],
                    [x + 1.0, y + 1.0, z],
                    [x + 1.0, y, z],
                ],
                [0.0, 0.0, -1.0],
                [[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]],
            ),
        }
    }
}

/// Chunk neighbor data for cross-chunk face culling
pub struct ChunkNeighbors {
    pub pos_x: Option<Vec<u16>>,
    pub neg_x: Option<Vec<u16>>,
    pub pos_z: Option<Vec<u16>>,
    pub neg_z: Option<Vec<u16>>,
}

impl Default for ChunkNeighbors {
    fn default() -> Self {
        Self {
            pos_x: None,
            neg_x: None,
            pos_z: None,
            neg_z: None,
        }
    }
}
