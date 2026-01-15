//! # Engine Configuration
//! 
//! Configuration parsed from Java-provided JSON.

use serde::{Deserialize, Serialize};

/// Render mode options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RenderMode {
    Vulkan,
    Opengl,
    Hybrid,
}

impl Default for RenderMode {
    fn default() -> Self {
        RenderMode::Hybrid
    }
}

/// Engine configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct EngineConfig {
    /// Render mode
    #[serde(rename = "renderMode")]
    pub render_mode: RenderMode,
    
    /// Maximum off-heap memory in MB
    #[serde(rename = "maxOffheapMB")]
    pub max_offheap_mb: u64,
    
    /// Enable VSync
    pub vsync: bool,
    
    /// Maximum FPS (0 = unlimited)
    #[serde(rename = "maxFps")]
    pub max_fps: u32,
    
    /// Render scale (1.0 = native)
    #[serde(rename = "renderScale")]
    pub render_scale: f32,
    
    /// Enable async chunk meshing
    #[serde(rename = "asyncChunks")]
    pub async_chunks: bool,
    
    /// Chunk mesh thread count
    #[serde(rename = "meshThreads")]
    pub mesh_threads: u32,
    
    /// Enable validation layers (debug)
    #[serde(rename = "validationLayers")]
    pub validation_layers: bool,
    
    /// Enable ECS profiling
    #[serde(rename = "ecsProfiling")]
    pub ecs_profiling: bool,
    
    /// Audio volume (0.0 - 1.0)
    #[serde(rename = "masterVolume")]
    pub master_volume: f32,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            render_mode: RenderMode::Hybrid,
            max_offheap_mb: 512,
            vsync: true,
            max_fps: 0,
            render_scale: 1.0,
            async_chunks: true,
            mesh_threads: 4,
            validation_layers: false,
            ecs_profiling: false,
            master_volume: 1.0,
        }
    }
}

impl EngineConfig {
    /// Parse config from bytes (JSON)
    pub fn from_bytes(data: &[u8]) -> Result<Self, String> {
        if data.is_empty() {
            log::warn!("Empty config data, using defaults");
            return Ok(Self::default());
        }
        
        serde_json::from_slice(data)
            .map_err(|e| format!("Config parse error: {}", e))
    }
    
    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_default()
    }
}
