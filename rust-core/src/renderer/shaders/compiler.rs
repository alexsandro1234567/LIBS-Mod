//! # Shader Compiler
//! 
//! GLSL to SPIR-V compilation using naga.

use super::{ShaderStage, ShaderError};
use naga::front::glsl;
use naga::back::spv;
use std::path::Path;

/// Shader compiler using naga
pub struct ShaderCompiler {
    /// Parser options
    options: glsl::Options,
}

impl ShaderCompiler {
    /// Create a new shader compiler
    pub fn new() -> Self {
        let options = glsl::Options::from(naga::ShaderStage::Vertex); // Stage updated per compile
        
        Self { options }
    }
    
    /// Compile GLSL source to SPIR-V
    pub fn compile(
        &self,
        source: &str,
        stage: ShaderStage,
        name: &str,
    ) -> Result<Vec<u32>, ShaderError> {
        // Preprocess (handle includes)
        let preprocessed = Self::preprocess_source(source, name)
            .map_err(|e| ShaderError::CompilationFailed(e))?;
        
        let naga_stage = Self::stage_to_naga(stage);
        
        // Configure options for this stage
        let mut options = glsl::Options::from(naga_stage);
        options.defines = self.options.defines.clone();
        
        // Create parser
        let mut parser = glsl::Frontend::default();
        
        // Parse module
        let module = parser.parse(&options, &preprocessed)
            .map_err(|e| {
                ShaderError::CompilationFailed(format!("Parse error in {}: {:?}", name, e))
            })?;
            
        // Validate module
        let mut validator = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        );
        
        let info = validator.validate(&module)
             .map_err(|e| ShaderError::CompilationFailed(format!("Validation error: {:?}", e)))?;
            
        // Generate SPIR-V
        let mut spv_options = spv::Options::default();
        spv_options.lang_version = (1, 5);
        spv_options.flags.insert(spv::WriterFlags::DEBUG);
        
        // Note: write_vec takes (module, info, options, pipeline_options)
        let spv = spv::write_vec(&module, &info, &spv_options, None)
            .map_err(|e| ShaderError::CompilationFailed(format!("SPIR-V generation error: {:?}", e)))?;
            
        Ok(spv)
    }
    
    /// Compile output to assembly (stub)
    pub fn compile_to_assembly(
        &self,
        _source: &str,
        _stage: ShaderStage,
        _name: &str,
    ) -> Result<String, ShaderError> {
        Err(ShaderError::CompilationFailed("Assembly generation not supported with naga backend".into()))
    }
    
    /// Preprocess shader source (expand includes)
    pub fn preprocess(
        &self,
        source: &str,
        _stage: ShaderStage,
        name: &str,
    ) -> Result<String, ShaderError> {
        Self::preprocess_source(source, name)
            .map_err(|e| ShaderError::CompilationFailed(e))
    }
    
    /// Manually resolve #include directives
    fn preprocess_source(source: &str, current_file: &str) -> Result<String, String> {
        let mut processed = String::new();
        
        for line in source.lines() {
            if line.trim().starts_with("#include") {
                // Parse include path
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() < 2 {
                    return Err(format!("Invalid include directive: {}", line));
                }
                
                let include_path = parts[1].trim_matches(|c| c == '"' || c == '<' || c == '>');
                
                // Resolve content
                let content = Self::resolve_include(include_path, current_file)?;
                
                // Recursively preprocess the included content
                let processed_include = Self::preprocess_source(&content, include_path)?;
                processed.push_str(&processed_include);
                processed.push('\n');
            } else {
                processed.push_str(line);
                processed.push('\n');
            }
        }
        
        Ok(processed)
    }
    
    /// Resolve include file content
    fn resolve_include(name: &str, current_file: &str) -> Result<String, String> {
        // Standard include paths
        let include_paths = [
            "shaders/",
            "shaders/include/",
            "assets/shaders/",
        ];
        
        // Try absolute/base paths
        for base_path in &include_paths {
            let full_path = format!("{}{}", base_path, name);
            if let Ok(content) = std::fs::read_to_string(&full_path) {
                return Ok(content);
            }
        }
        
        // Try relative to current file
        if let Some(parent) = Path::new(current_file).parent() {
            let full_path = parent.join(name);
            if let Ok(content) = std::fs::read_to_string(&full_path) {
                return Ok(content);
            }
        }
        
        Err(format!("Include file not found: {}", name))
    }
    
    /// Add a macro definition
    pub fn define_macro(&mut self, name: &str, value: Option<&str>) {
        if let Some(val) = value {
            self.options.defines.insert(name.to_string(), val.to_string());
        } else {
             self.options.defines.insert(name.to_string(), "1".to_string());
        }
    }
    
    /// Set optimization level
    pub fn set_optimization(&mut self, _level: OptimizationLevel) {
        // Naga doesn't have granular optimization levels like shaderc
    }
    
    /// Convert shader stage to naga stage
    fn stage_to_naga(stage: ShaderStage) -> naga::ShaderStage {
        match stage {
            ShaderStage::Vertex => naga::ShaderStage::Vertex,
            ShaderStage::Fragment => naga::ShaderStage::Fragment,
            ShaderStage::Compute => naga::ShaderStage::Compute,
            _ => naga::ShaderStage::Compute, // Fallback
        }
    }
}

impl Default for ShaderCompiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Optimization level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationLevel {
    None,
    Size,
    Performance,
}

/// Shader macro definition
#[derive(Debug, Clone)]
pub struct MacroDefinition {
    pub name: String,
    pub value: Option<String>,
}

/// Shader compilation configuration
#[derive(Debug, Clone)]
pub struct CompileConfig {
    pub optimization: OptimizationLevel,
    pub debug_info: bool,
    pub macros: Vec<MacroDefinition>,
    pub include_paths: Vec<String>,
}

impl Default for CompileConfig {
    fn default() -> Self {
        Self {
            optimization: OptimizationLevel::Performance,
            debug_info: cfg!(debug_assertions),
            macros: Vec::new(),
            include_paths: Vec::new(),
        }
    }
}
