//! # Shader Reflection
//! 
//! SPIR-V reflection for extracting shader interface information.

use super::ShaderError;
use std::collections::HashMap;

/// Shader reflection data
#[derive(Debug, Clone, Default)]
pub struct ShaderReflection {
    /// Descriptor set bindings
    pub descriptor_sets: HashMap<u32, Vec<DescriptorBinding>>,
    /// Push constant ranges
    pub push_constants: Vec<PushConstantRange>,
    /// Input variables (for vertex shaders)
    pub inputs: Vec<ShaderVariable>,
    /// Output variables (for fragment shaders)
    pub outputs: Vec<ShaderVariable>,
    /// Workgroup size (for compute shaders)
    pub workgroup_size: Option<[u32; 3]>,
    /// Specialization constants
    pub spec_constants: Vec<SpecializationConstant>,
}

impl ShaderReflection {
    /// Create reflection data from SPIR-V bytecode
    pub fn from_spirv(spirv: &[u32]) -> Result<Self, ShaderError> {
        let module = spirv_reflect::ShaderModule::load_u32_data(spirv)
            .map_err(|e| ShaderError::ReflectionError(e.to_string()))?;
        
        let mut reflection = ShaderReflection::default();
        
        // Extract descriptor bindings
        if let Ok(bindings) = module.enumerate_descriptor_bindings(None) {
            for binding in bindings {
                let descriptor = DescriptorBinding {
                    binding: binding.binding,
                    descriptor_type: Self::convert_descriptor_type(binding.descriptor_type),
                    count: binding.count,
                    name: binding.name.clone(),
                    stage_flags: 0, // Would need to track from shader stage
                };
                
                reflection.descriptor_sets
                    .entry(binding.set)
                    .or_insert_with(Vec::new)
                    .push(descriptor);
            }
        }
        
        // Extract push constants
        if let Ok(blocks) = module.enumerate_push_constant_blocks(None) {
            for block in blocks {
                reflection.push_constants.push(PushConstantRange {
                    offset: block.offset,
                    size: block.size,
                    name: block.name.clone(),
                    members: block.members.iter().map(|m| PushConstantMember {
                        name: m.name.clone(),
                        offset: m.offset,
                        size: m.size,
                        type_name: Self::type_description_to_string(&m.type_description),
                    }).collect(),
                });
            }
        }
        
        // Extract input variables
        if let Ok(inputs) = module.enumerate_input_variables(None) {
            for input in inputs {
                reflection.inputs.push(ShaderVariable {
                    location: input.location,
                    name: input.name.clone(),
                    format: Self::format_from_type(&input.type_description),
                });
            }
        }
        
        // Extract output variables
        if let Ok(outputs) = module.enumerate_output_variables(None) {
            for output in outputs {
                reflection.outputs.push(ShaderVariable {
                    location: output.location,
                    name: output.name.clone(),
                    format: Self::format_from_type(&output.type_description),
                });
            }
        }
        
        // Extract specialization constants
        /*
        if let Ok(spec_consts) = module.enumerate_specialization_constants() {
            for sc in spec_consts {
                reflection.spec_constants.push(SpecializationConstant {
                    id: sc.constant_id,
                    name: sc.name.clone(),
                    size: 4, // Default size, would need type info
                });
            }
        }
        */
        
        Ok(reflection)
    }
    
    /// Convert spirv-reflect descriptor type to our type
    fn convert_descriptor_type(dt: spirv_reflect::types::ReflectDescriptorType) -> DescriptorType {
        match dt {
            spirv_reflect::types::ReflectDescriptorType::Sampler => DescriptorType::Sampler,
            spirv_reflect::types::ReflectDescriptorType::CombinedImageSampler => DescriptorType::CombinedImageSampler,
            spirv_reflect::types::ReflectDescriptorType::SampledImage => DescriptorType::SampledImage,
            spirv_reflect::types::ReflectDescriptorType::StorageImage => DescriptorType::StorageImage,
            spirv_reflect::types::ReflectDescriptorType::UniformTexelBuffer => DescriptorType::UniformTexelBuffer,
            spirv_reflect::types::ReflectDescriptorType::StorageTexelBuffer => DescriptorType::StorageTexelBuffer,
            spirv_reflect::types::ReflectDescriptorType::UniformBuffer => DescriptorType::UniformBuffer,
            spirv_reflect::types::ReflectDescriptorType::StorageBuffer => DescriptorType::StorageBuffer,
            spirv_reflect::types::ReflectDescriptorType::UniformBufferDynamic => DescriptorType::UniformBufferDynamic,
            spirv_reflect::types::ReflectDescriptorType::StorageBufferDynamic => DescriptorType::StorageBufferDynamic,
            spirv_reflect::types::ReflectDescriptorType::InputAttachment => DescriptorType::InputAttachment,
            spirv_reflect::types::ReflectDescriptorType::AccelerationStructureNV => DescriptorType::AccelerationStructure,
            _ => DescriptorType::Unknown,
        }
    }
    
    /// Convert type description to string
    fn type_description_to_string(td: &Option<spirv_reflect::types::ReflectTypeDescription>) -> String {
        match td {
            Some(desc) => desc.type_name.clone(),
            None => "unknown".to_string(),
        }
    }
    
    /// Get format from type description
    fn format_from_type(td: &Option<spirv_reflect::types::ReflectTypeDescription>) -> VariableFormat {
        match td {
            Some(desc) => {
                let traits = &desc.traits;
                match (traits.numeric.scalar.width, traits.numeric.vector.component_count) {
                    (32, 1) => VariableFormat::Float,
                    (32, 2) => VariableFormat::Vec2,
                    (32, 3) => VariableFormat::Vec3,
                    (32, 4) => VariableFormat::Vec4,
                    _ => VariableFormat::Unknown,
                }
            }
            None => VariableFormat::Unknown,
        }
    }
}

/// Combined program reflection data
#[derive(Debug, Clone, Default)]
pub struct ProgramReflection {
    /// Descriptor set bindings (merged from all stages)
    pub descriptor_sets: HashMap<u32, Vec<DescriptorBinding>>,
    /// Push constant ranges (merged from all stages)
    pub push_constants: Vec<PushConstantRange>,
}

/// Descriptor binding information
#[derive(Debug, Clone)]
pub struct DescriptorBinding {
    /// Binding number
    pub binding: u32,
    /// Descriptor type
    pub descriptor_type: DescriptorType,
    /// Array count (1 for non-arrays)
    pub count: u32,
    /// Binding name
    pub name: String,
    /// Shader stage flags
    pub stage_flags: u32,
}

/// Descriptor types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DescriptorType {
    Sampler,
    CombinedImageSampler,
    SampledImage,
    StorageImage,
    UniformTexelBuffer,
    StorageTexelBuffer,
    UniformBuffer,
    StorageBuffer,
    UniformBufferDynamic,
    StorageBufferDynamic,
    InputAttachment,
    AccelerationStructure,
    Unknown,
}

/// Push constant range
#[derive(Debug, Clone)]
pub struct PushConstantRange {
    /// Offset in bytes
    pub offset: u32,
    /// Size in bytes
    pub size: u32,
    /// Block name
    pub name: String,
    /// Members
    pub members: Vec<PushConstantMember>,
}

/// Push constant member
#[derive(Debug, Clone)]
pub struct PushConstantMember {
    /// Member name
    pub name: String,
    /// Offset within block
    pub offset: u32,
    /// Size in bytes
    pub size: u32,
    /// Type name
    pub type_name: String,
}

/// Shader input/output variable
#[derive(Debug, Clone)]
pub struct ShaderVariable {
    /// Location
    pub location: u32,
    /// Variable name
    pub name: String,
    /// Variable format
    pub format: VariableFormat,
}

/// Variable format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableFormat {
    Float,
    Vec2,
    Vec3,
    Vec4,
    Int,
    IVec2,
    IVec3,
    IVec4,
    UInt,
    UVec2,
    UVec3,
    UVec4,
    Mat3,
    Mat4,
    Unknown,
}

/// Specialization constant
#[derive(Debug, Clone)]
pub struct SpecializationConstant {
    /// Constant ID
    pub id: u32,
    /// Constant name
    pub name: String,
    /// Size in bytes
    pub size: u32,
}
