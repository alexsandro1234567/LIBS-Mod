//! # The Weaver - Compatibility Layer
//! 
//! Dynamic sandboxing and fallback detection for mod compatibility.
//! Handles legacy entities, fallback rendering, and user notifications.

use std::sync::Arc;
use std::collections::{HashMap, HashSet};
use parking_lot::RwLock;

/// Compatibility level for a mod/feature
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompatLevel {
    /// Fully compatible with LIBS
    Full,
    /// Partial compatibility, some fallbacks needed
    Partial,
    /// Requires legacy rendering
    Legacy,
    /// Incompatible, will be sandboxed
    Sandboxed,
    /// Unknown, needs testing
    Unknown,
}

/// Entity render mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderMode {
    /// Use LIBS Vulkan renderer
    Vulkan,
    /// Use hybrid OpenGL+Vulkan
    Hybrid,
    /// Use pure OpenGL (legacy)
    OpenGL,
    /// Skip rendering (invisible)
    Skip,
}

/// Legacy flag for per-entity compatibility
#[derive(Debug, Clone)]
pub struct LegacyFlags {
    /// Use legacy renderer
    pub legacy_render: bool,
    /// Use legacy physics
    pub legacy_physics: bool,
    /// Use legacy networking
    pub legacy_network: bool,
    /// Custom NBT handling
    pub custom_nbt: bool,
    /// Render priority override
    pub render_priority: Option<i32>,
    /// Force OpenGL
    pub force_opengl: bool,
}

impl Default for LegacyFlags {
    fn default() -> Self {
        Self {
            legacy_render: false,
            legacy_physics: false,
            legacy_network: false,
            custom_nbt: false,
            render_priority: None,
            force_opengl: false,
        }
    }
}

impl LegacyFlags {
    pub fn fully_legacy() -> Self {
        Self {
            legacy_render: true,
            legacy_physics: true,
            legacy_network: true,
            custom_nbt: true,
            render_priority: None,
            force_opengl: true,
        }
    }
    
    pub fn needs_legacy(&self) -> bool {
        self.legacy_render || self.legacy_physics || self.force_opengl
    }
}

/// Mod compatibility info
#[derive(Debug, Clone)]
pub struct ModCompat {
    pub mod_id: String,
    pub mod_name: String,
    pub level: CompatLevel,
    pub flags: LegacyFlags,
    pub notes: String,
    pub tested_version: Option<String>,
}

/// Entity compatibility info
#[derive(Debug, Clone)]
pub struct EntityCompat {
    pub entity_type: String,
    pub mod_id: String,
    pub flags: LegacyFlags,
    pub render_mode: RenderMode,
}

/// User notification
#[derive(Debug, Clone)]
pub struct Notification {
    pub id: u64,
    pub title: String,
    pub message: String,
    pub level: NotificationLevel,
    pub timestamp: u64,
    pub dismissible: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationLevel {
    Info,
    Warning,
    Error,
    Performance,
}

/// Fallback trigger event
#[derive(Debug, Clone)]
pub struct FallbackEvent {
    pub reason: String,
    pub entity_type: Option<String>,
    pub mod_id: Option<String>,
    pub suggested_action: String,
}

/// The Weaver - Compatibility Manager
pub struct TheWeaver {
    /// Known mod compatibility database
    mod_compat: HashMap<String, ModCompat>,
    /// Per-entity type compatibility
    entity_compat: HashMap<String, EntityCompat>,
    /// Active fallbacks
    active_fallbacks: HashSet<String>,
    /// Pending notifications
    notifications: Vec<Notification>,
    /// Notification ID counter
    next_notification_id: u64,
    /// Detected mods
    detected_mods: Vec<String>,
    /// Statistics
    stats: WeaverStats,
    /// Initialized
    initialized: bool,
}

/// Weaver statistics
#[derive(Debug, Default, Clone)]
pub struct WeaverStats {
    pub mods_detected: u32,
    pub full_compat: u32,
    pub partial_compat: u32,
    pub legacy_entities: u32,
    pub sandboxed: u32,
    pub fallbacks_triggered: u32,
    pub notifications_sent: u32,
}

impl TheWeaver {
    pub fn new() -> Self {
        log::info!("Initializing The Weaver compatibility layer");
        
        let mut weaver = Self {
            mod_compat: HashMap::with_capacity(256),
            entity_compat: HashMap::with_capacity(1024),
            active_fallbacks: HashSet::new(),
            notifications: Vec::new(),
            next_notification_id: 1,
            detected_mods: Vec::new(),
            stats: WeaverStats::default(),
            initialized: false,
        };
        
        // Register known mod compatibilities
        weaver.register_known_mods();
        
        weaver.initialized = true;
        weaver
    }
    
    /// Register known mod compatibilities
    fn register_known_mods(&mut self) {
        // OptiFine - Conflict, must disable certain features
        self.mod_compat.insert("optifine".to_string(), ModCompat {
            mod_id: "optifine".to_string(),
            mod_name: "OptiFine".to_string(),
            level: CompatLevel::Sandboxed,
            flags: LegacyFlags::fully_legacy(),
            notes: "LIBS replaces OptiFine functionality".to_string(),
            tested_version: Some("HD U".to_string()),
        });
        
        // Sodium - Partial, can coexist
        self.mod_compat.insert("sodium".to_string(), ModCompat {
            mod_id: "sodium".to_string(),
            mod_name: "Sodium".to_string(),
            level: CompatLevel::Partial,
            flags: LegacyFlags {
                legacy_render: true,
                ..Default::default()
            },
            notes: "Disable Sodium chunk rendering for best results".to_string(),
            tested_version: Some("0.5".to_string()),
        });
        
        // JEI/REI - Full compat
        self.mod_compat.insert("jei".to_string(), ModCompat {
            mod_id: "jei".to_string(),
            mod_name: "Just Enough Items".to_string(),
            level: CompatLevel::Full,
            flags: LegacyFlags::default(),
            notes: "Fully compatible".to_string(),
            tested_version: Some("15".to_string()),
        });
        
        // Create - Partial due to custom rendering
        self.mod_compat.insert("create".to_string(), ModCompat {
            mod_id: "create".to_string(),
            mod_name: "Create".to_string(),
            level: CompatLevel::Partial,
            flags: LegacyFlags {
                legacy_render: false,
                custom_nbt: true,
                ..Default::default()
            },
            notes: "Contraptions use hybrid rendering".to_string(),
            tested_version: Some("0.5".to_string()),
        });
        
        // Mekanism - Partial
        self.mod_compat.insert("mekanism".to_string(), ModCompat {
            mod_id: "mekanism".to_string(),
            mod_name: "Mekanism".to_string(),
            level: CompatLevel::Partial,
            flags: LegacyFlags {
                legacy_physics: true,
                ..Default::default()
            },
            notes: "Custom fluid rendering uses fallback".to_string(),
            tested_version: Some("10".to_string()),
        });
        
        // Thaumcraft - Legacy (old mod)
        self.mod_compat.insert("thaumcraft".to_string(), ModCompat {
            mod_id: "thaumcraft".to_string(),
            mod_name: "Thaumcraft".to_string(),
            level: CompatLevel::Legacy,
            flags: LegacyFlags::fully_legacy(),
            notes: "Uses legacy rendering for all entities".to_string(),
            tested_version: Some("6".to_string()),
        });
    }
    
    /// Detect installed mods
    pub fn detect_mods(&mut self, mod_list: &[String]) {
        self.detected_mods = mod_list.to_vec();
        self.stats.mods_detected = mod_list.len() as u32;
        
        for mod_id in mod_list {
            if let Some(compat) = self.mod_compat.get(mod_id) {
                match compat.level {
                    CompatLevel::Full => self.stats.full_compat += 1,
                    CompatLevel::Partial | CompatLevel::Legacy => self.stats.partial_compat += 1,
                    CompatLevel::Sandboxed => {
                        self.stats.sandboxed += 1;
                        self.send_notification(
                            "Mod Sandboxed",
                            &format!("{} has been sandboxed for compatibility", compat.mod_name),
                            NotificationLevel::Warning,
                        );
                    }
                    CompatLevel::Unknown => {}
                }
            } else {
                // Unknown mod - mark for testing
                self.mod_compat.insert(mod_id.clone(), ModCompat {
                    mod_id: mod_id.clone(),
                    mod_name: mod_id.clone(),
                    level: CompatLevel::Unknown,
                    flags: LegacyFlags::default(),
                    notes: "Unknown mod - monitoring for issues".to_string(),
                    tested_version: None,
                });
            }
        }
        
        log::info!("Detected {} mods ({} full, {} partial, {} sandboxed)",
            self.stats.mods_detected,
            self.stats.full_compat,
            self.stats.partial_compat,
            self.stats.sandboxed);
    }
    
    /// Get render mode for entity type
    pub fn get_entity_render_mode(&self, entity_type: &str) -> RenderMode {
        if let Some(compat) = self.entity_compat.get(entity_type) {
            return compat.render_mode;
        }
        
        // Check if entity's mod needs legacy
        let mod_id = entity_type.split(':').next().unwrap_or("minecraft");
        if let Some(mod_compat) = self.mod_compat.get(mod_id) {
            if mod_compat.flags.force_opengl {
                return RenderMode::OpenGL;
            }
            if mod_compat.level == CompatLevel::Legacy {
                return RenderMode::OpenGL;
            }
            if mod_compat.level == CompatLevel::Partial {
                return RenderMode::Hybrid;
            }
        }
        
        RenderMode::Vulkan
    }
    
    /// Get legacy flags for entity
    pub fn get_entity_flags(&self, entity_type: &str) -> LegacyFlags {
        if let Some(compat) = self.entity_compat.get(entity_type) {
            return compat.flags.clone();
        }
        
        // Check mod-level flags
        let mod_id = entity_type.split(':').next().unwrap_or("minecraft");
        if let Some(mod_compat) = self.mod_compat.get(mod_id) {
            return mod_compat.flags.clone();
        }
        
        LegacyFlags::default()
    }
    
    /// Register entity-specific compatibility
    pub fn register_entity(&mut self, entity_type: &str, flags: LegacyFlags, render_mode: RenderMode) {
        let mod_id = entity_type.split(':').next().unwrap_or("minecraft").to_string();
        
        self.entity_compat.insert(entity_type.to_string(), EntityCompat {
            entity_type: entity_type.to_string(),
            mod_id,
            flags: flags.clone(),
            render_mode,
        });
        
        if flags.needs_legacy() {
            self.stats.legacy_entities += 1;
        }
    }
    
    /// Trigger fallback rendering
    pub fn trigger_fallback(&mut self, reason: &str, entity_type: Option<&str>, mod_id: Option<&str>) {
        let key = format!("{:?}:{:?}", entity_type, mod_id);
        
        if self.active_fallbacks.insert(key.clone()) {
            self.stats.fallbacks_triggered += 1;
            
            let event = FallbackEvent {
                reason: reason.to_string(),
                entity_type: entity_type.map(|s| s.to_string()),
                mod_id: mod_id.map(|s| s.to_string()),
                suggested_action: "Using legacy renderer as fallback".to_string(),
            };
            
            log::warn!("Fallback triggered: {} ({:?})", reason, entity_type);
            
            // Update entity compat if specific entity caused fallback
            if let Some(entity) = entity_type {
                self.register_entity(entity, LegacyFlags::fully_legacy(), RenderMode::OpenGL);
            }
        }
    }
    
    /// Check if fallback is active
    pub fn is_fallback_active(&self, entity_type: &str) -> bool {
        let key = format!("{:?}:None", Some(entity_type));
        self.active_fallbacks.contains(&key)
    }
    
    /// Send user notification
    pub fn send_notification(&mut self, title: &str, message: &str, level: NotificationLevel) {
        let notification = Notification {
            id: self.next_notification_id,
            title: title.to_string(),
            message: message.to_string(),
            level,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            dismissible: true,
        };
        
        self.next_notification_id += 1;
        self.stats.notifications_sent += 1;
        self.notifications.push(notification);
        
        log::info!("Notification: [{}] {}", title, message);
    }
    
    /// Get pending notifications
    pub fn get_notifications(&self) -> &[Notification] {
        &self.notifications
    }
    
    /// Dismiss notification
    pub fn dismiss_notification(&mut self, id: u64) {
        self.notifications.retain(|n| n.id != id);
    }
    
    /// Clear all notifications
    pub fn clear_notifications(&mut self) {
        self.notifications.clear();
    }
    
    /// Get mod compatibility level
    pub fn get_mod_compat(&self, mod_id: &str) -> CompatLevel {
        self.mod_compat.get(mod_id)
            .map(|c| c.level)
            .unwrap_or(CompatLevel::Unknown)
    }
    
    /// Check if mod is compatible
    pub fn is_mod_compatible(&self, mod_id: &str) -> bool {
        matches!(
            self.get_mod_compat(mod_id),
            CompatLevel::Full | CompatLevel::Partial
        )
    }
    
    /// Get statistics
    pub fn stats(&self) -> &WeaverStats {
        &self.stats
    }
    
    /// Generate compatibility report
    pub fn generate_report(&self) -> String {
        let mut report = String::new();
        report.push_str("=== LIBS Compatibility Report ===\n\n");
        report.push_str(&format!("Mods Detected: {}\n", self.stats.mods_detected));
        report.push_str(&format!("Full Compatibility: {}\n", self.stats.full_compat));
        report.push_str(&format!("Partial Compatibility: {}\n", self.stats.partial_compat));
        report.push_str(&format!("Sandboxed: {}\n", self.stats.sandboxed));
        report.push_str(&format!("Legacy Entities: {}\n", self.stats.legacy_entities));
        report.push_str(&format!("Fallbacks Triggered: {}\n\n", self.stats.fallbacks_triggered));
        
        report.push_str("--- Mod Details ---\n");
        for (mod_id, compat) in &self.mod_compat {
            if self.detected_mods.contains(mod_id) {
                report.push_str(&format!(
                    "{}: {:?} - {}\n",
                    compat.mod_name, compat.level, compat.notes
                ));
            }
        }
        
        report
    }
    
    /// Clear all data
    pub fn clear(&mut self) {
        self.entity_compat.clear();
        self.active_fallbacks.clear();
        self.notifications.clear();
        self.detected_mods.clear();
        self.stats = WeaverStats::default();
    }
    
    /// Shutdown
    pub fn shutdown(&mut self) {
        self.clear();
        self.initialized = false;
        log::info!("The Weaver shutdown");
    }
}

impl Default for TheWeaver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_weaver_creation() {
        let weaver = TheWeaver::new();
        assert!(weaver.initialized);
    }
    
    #[test]
    fn test_mod_detection() {
        let mut weaver = TheWeaver::new();
        weaver.detect_mods(&["jei".to_string(), "create".to_string()]);
        
        assert_eq!(weaver.stats.mods_detected, 2);
        assert!(weaver.is_mod_compatible("jei"));
        assert!(weaver.is_mod_compatible("create"));
    }
    
    #[test]
    fn test_entity_render_mode() {
        let weaver = TheWeaver::new();
        
        // Vanilla entities should use Vulkan
        assert_eq!(weaver.get_entity_render_mode("minecraft:pig"), RenderMode::Vulkan);
        
        // Thaumcraft entities should use OpenGL
        assert_eq!(weaver.get_entity_render_mode("thaumcraft:golem"), RenderMode::OpenGL);
    }
    
    #[test]
    fn test_fallback() {
        let mut weaver = TheWeaver::new();
        weaver.trigger_fallback("Test", Some("test:entity"), None);
        
        assert!(weaver.is_fallback_active("test:entity"));
        assert_eq!(weaver.stats.fallbacks_triggered, 1);
    }
}
