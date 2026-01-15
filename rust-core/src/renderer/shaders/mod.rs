//! # Shader Compilation System
//! 
//! Runtime and build-time shader compilation for Vulkan.
//! Supports GLSL, HLSL, and SPIR-V with hot-reloading.

pub mod compiler;
pub mod cache;
pub mod reflection;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use ash::vk;

pub use compiler::*;
pub use cache::*;
pub use reflection::*;

/// Shader stage types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShaderStage {
    Vertex,
    Fragment,
    Compute,
    Geometry,
    TessControl,
    TessEvaluation,
    Task,
    Mesh,
    RayGen,
    RayMiss,
    RayClosestHit,
    RayAnyHit,
    RayIntersection,
}

impl ShaderStage {
    /// Convert to Vulkan shader stage flags
    pub fn to_vk_flags(&self) -> vk::ShaderStageFlags {
        match self {
            ShaderStage::Vertex => vk::ShaderStageFlags::VERTEX,
            ShaderStage::Fragment => vk::ShaderStageFlags::FRAGMENT,
            ShaderStage::Compute => vk::ShaderStageFlags::COMPUTE,
            ShaderStage::Geometry => vk::ShaderStageFlags::GEOMETRY,
            ShaderStage::TessControl => vk::ShaderStageFlags::TESSELLATION_CONTROL,
            ShaderStage::TessEvaluation => vk::ShaderStageFlags::TESSELLATION_EVALUATION,
            ShaderStage::Task => vk::ShaderStageFlags::TASK_EXT,
            ShaderStage::Mesh => vk::ShaderStageFlags::MESH_EXT,
            ShaderStage::RayGen => vk::ShaderStageFlags::RAYGEN_KHR,
            ShaderStage::RayMiss => vk::ShaderStageFlags::MISS_KHR,
            ShaderStage::RayClosestHit => vk::ShaderStageFlags::CLOSEST_HIT_KHR,
            ShaderStage::RayAnyHit => vk::ShaderStageFlags::ANY_HIT_KHR,
            ShaderStage::RayIntersection => vk::ShaderStageFlags::INTERSECTION_KHR,
        }
    }
    
    /// Get file extension for shader stage
    pub fn extension(&self) -> &'static str {
        match self {
            ShaderStage::Vertex => "vert",
            ShaderStage::Fragment => "frag",
            ShaderStage::Compute => "comp",
            ShaderStage::Geometry => "geom",
            ShaderStage::TessControl => "tesc",
            ShaderStage::TessEvaluation => "tese",
            ShaderStage::Task => "task",
            ShaderStage::Mesh => "mesh",
            ShaderStage::RayGen => "rgen",
            ShaderStage::RayMiss => "rmiss",
            ShaderStage::RayClosestHit => "rchit",
            ShaderStage::RayAnyHit => "rahit",
            ShaderStage::RayIntersection => "rint",
        }
    }
}

/// Compiled shader module
#[derive(Clone)]
pub struct ShaderModule {
    /// Shader name/identifier
    pub name: String,
    /// Shader stage
    pub stage: ShaderStage,
    /// SPIR-V bytecode
    pub spirv: Vec<u32>,
    /// Entry point name
    pub entry_point: String,
    /// Reflection data
    pub reflection: ShaderReflection,
    /// Source file path (for hot-reloading)
    pub source_path: Option<PathBuf>,
    /// Last modification time
    pub last_modified: Option<std::time::SystemTime>,
}

/// Shader program (multiple stages)
pub struct ShaderProgram {
    /// Program name
    pub name: String,
    /// Shader modules by stage
    pub modules: HashMap<ShaderStage, Arc<ShaderModule>>,
    /// Combined reflection data
    pub reflection: ProgramReflection,
}

impl ShaderProgram {
    /// Create a new shader program
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            modules: HashMap::new(),
            reflection: ProgramReflection::default(),
        }
    }
    
    /// Add a shader module
    pub fn add_module(&mut self, module: Arc<ShaderModule>) {
        self.modules.insert(module.stage, module);
        self.update_reflection();
    }
    
    /// Update combined reflection data
    fn update_reflection(&mut self) {
        self.reflection = ProgramReflection::default();
        
        for module in self.modules.values() {
            // Merge descriptor sets
            for (set, bindings) in &module.reflection.descriptor_sets {
                let entry = self.reflection.descriptor_sets.entry(*set).or_insert_with(Vec::new);
                for binding in bindings {
                    if !entry.iter().any(|b| b.binding == binding.binding) {
                        entry.push(binding.clone());
                    }
                }
            }
            
            // Merge push constants
            for pc in &module.reflection.push_constants {
                if !self.reflection.push_constants.iter().any(|p| p.offset == pc.offset) {
                    self.reflection.push_constants.push(pc.clone());
                }
            }
        }
    }
    
    /// Check if program has all required stages for graphics pipeline
    pub fn is_graphics_complete(&self) -> bool {
        self.modules.contains_key(&ShaderStage::Vertex) &&
        self.modules.contains_key(&ShaderStage::Fragment)
    }
    
    /// Check if program is a compute shader
    pub fn is_compute(&self) -> bool {
        self.modules.contains_key(&ShaderStage::Compute) && self.modules.len() == 1
    }
    
    /// Check if program uses mesh shaders
    pub fn uses_mesh_shaders(&self) -> bool {
        self.modules.contains_key(&ShaderStage::Mesh)
    }
}

/// Shader manager for loading and caching shaders
pub struct ShaderManager {
    /// Shader compiler
    compiler: ShaderCompiler,
    /// Shader cache
    cache: RwLock<ShaderCache>,
    /// Loaded modules
    modules: RwLock<HashMap<String, Arc<ShaderModule>>>,
    /// Loaded programs
    programs: RwLock<HashMap<String, Arc<ShaderProgram>>>,
    /// Shader search paths
    search_paths: Vec<PathBuf>,
    /// Hot-reload enabled
    hot_reload: bool,
}

impl ShaderManager {
    /// Create a new shader manager
    pub fn new(cache_dir: Option<PathBuf>) -> Self {
        Self {
            compiler: ShaderCompiler::new(),
            cache: RwLock::new(ShaderCache::new(cache_dir)),
            modules: RwLock::new(HashMap::new()),
            programs: RwLock::new(HashMap::new()),
            search_paths: Vec::new(),
            hot_reload: cfg!(debug_assertions),
        }
    }
    
    /// Add a shader search path
    pub fn add_search_path<P: AsRef<Path>>(&mut self, path: P) {
        self.search_paths.push(path.as_ref().to_path_buf());
    }
    
    /// Load a shader module from source
    pub fn load_module(
        &self,
        name: &str,
        stage: ShaderStage,
        source: &str,
    ) -> Result<Arc<ShaderModule>, ShaderError> {
        // Check cache first
        let cache_key = format!("{}_{:?}", name, stage);
        if let Some(spirv) = self.cache.read().unwrap().get(&cache_key, source) {
            let reflection = ShaderReflection::from_spirv(&spirv)?;
            let module = Arc::new(ShaderModule {
                name: name.to_string(),
                stage,
                spirv,
                entry_point: "main".to_string(),
                reflection,
                source_path: None,
                last_modified: None,
            });
            
            self.modules.write().unwrap().insert(cache_key, module.clone());
            return Ok(module);
        }
        
        // Compile shader
        let spirv = self.compiler.compile(source, stage, name)?;
        
        // Cache compiled shader
        self.cache.write().unwrap().put(&cache_key, source, &spirv);
        
        // Create reflection data
        let reflection = ShaderReflection::from_spirv(&spirv)?;
        
        let module = Arc::new(ShaderModule {
            name: name.to_string(),
            stage,
            spirv,
            entry_point: "main".to_string(),
            reflection,
            source_path: None,
            last_modified: None,
        });
        
        self.modules.write().unwrap().insert(cache_key, module.clone());
        Ok(module)
    }
    
    /// Load a shader module from file
    pub fn load_module_from_file<P: AsRef<Path>>(
        &self,
        path: P,
        stage: ShaderStage,
    ) -> Result<Arc<ShaderModule>, ShaderError> {
        let path = self.resolve_path(path.as_ref())?;
        let source = std::fs::read_to_string(&path)
            .map_err(|e| ShaderError::IoError(e.to_string()))?;
        
        let name = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        
        let mut module = (*self.load_module(name, stage, &source)?).clone();
        module.source_path = Some(path.clone());
        module.last_modified = std::fs::metadata(&path).ok().and_then(|m| m.modified().ok());
        
        let module = Arc::new(module);
        let cache_key = format!("{}_{:?}", name, stage);
        self.modules.write().unwrap().insert(cache_key, module.clone());
        
        Ok(module)
    }
    
    /// Load a shader program from multiple files
    pub fn load_program(&self, name: &str, stages: &[(ShaderStage, &str)]) -> Result<Arc<ShaderProgram>, ShaderError> {
        let mut program = ShaderProgram::new(name);
        
        for (stage, source) in stages {
            let module = self.load_module(&format!("{}_{:?}", name, stage), *stage, source)?;
            program.add_module(module);
        }
        
        let program = Arc::new(program);
        self.programs.write().unwrap().insert(name.to_string(), program.clone());
        
        Ok(program)
    }
    
    /// Get a loaded module
    pub fn get_module(&self, name: &str, stage: ShaderStage) -> Option<Arc<ShaderModule>> {
        let cache_key = format!("{}_{:?}", name, stage);
        self.modules.read().unwrap().get(&cache_key).cloned()
    }
    
    /// Get a loaded program
    pub fn get_program(&self, name: &str) -> Option<Arc<ShaderProgram>> {
        self.programs.read().unwrap().get(name).cloned()
    }
    
    /// Check for shader changes and reload if necessary
    pub fn check_hot_reload(&self) -> Vec<String> {
        if !self.hot_reload {
            return Vec::new();
        }
        
        let mut reloaded = Vec::new();
        let modules = self.modules.read().unwrap();
        
        for (key, module) in modules.iter() {
            if let (Some(path), Some(last_modified)) = (&module.source_path, module.last_modified) {
                if let Ok(metadata) = std::fs::metadata(path) {
                    if let Ok(current_modified) = metadata.modified() {
                        if current_modified > last_modified {
                            reloaded.push(key.clone());
                        }
                    }
                }
            }
        }
        
        drop(modules);
        
        // Reload changed shaders
        for key in &reloaded {
            if let Some(module) = self.modules.read().unwrap().get(key) {
                if let Some(path) = &module.source_path {
                    let _ = self.load_module_from_file(path, module.stage);
                }
            }
        }
        
        reloaded
    }
    
    /// Resolve shader path using search paths
    fn resolve_path(&self, path: &Path) -> Result<PathBuf, ShaderError> {
        if path.exists() {
            return Ok(path.to_path_buf());
        }
        
        for search_path in &self.search_paths {
            let full_path = search_path.join(path);
            if full_path.exists() {
                return Ok(full_path);
            }
        }
        
        Err(ShaderError::FileNotFound(path.display().to_string()))
    }
}

/// Shader compilation/loading errors
#[derive(Debug)]
pub enum ShaderError {
    CompilationFailed(String),
    FileNotFound(String),
    IoError(String),
    InvalidSpirv(String),
    ReflectionError(String),
}

impl std::fmt::Display for ShaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShaderError::CompilationFailed(msg) => write!(f, "Shader compilation failed: {}", msg),
            ShaderError::FileNotFound(path) => write!(f, "Shader file not found: {}", path),
            ShaderError::IoError(msg) => write!(f, "IO error: {}", msg),
            ShaderError::InvalidSpirv(msg) => write!(f, "Invalid SPIR-V: {}", msg),
            ShaderError::ReflectionError(msg) => write!(f, "Reflection error: {}", msg),
        }
    }
}

impl std::error::Error for ShaderError {}
