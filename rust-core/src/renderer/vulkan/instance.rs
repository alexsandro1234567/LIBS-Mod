//! # Vulkan Instance
//! 
//! Vulkan instance creation and management.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use ash::{vk, Entry, Instance};

use super::{VulkanConfig, VulkanError};

/// Required validation layers
const VALIDATION_LAYERS: &[&str] = &["VK_LAYER_KHRONOS_validation"];

/// Required instance extensions
#[cfg(target_os = "windows")]
const REQUIRED_EXTENSIONS: &[&str] = &[
    "VK_KHR_surface",
    "VK_KHR_win32_surface",
];

#[cfg(target_os = "linux")]
const REQUIRED_EXTENSIONS: &[&str] = &[
    "VK_KHR_surface",
    "VK_KHR_xlib_surface",
    "VK_KHR_xcb_surface",
];

#[cfg(target_os = "macos")]
const REQUIRED_EXTENSIONS: &[&str] = &[
    "VK_KHR_surface",
    "VK_EXT_metal_surface",
    "VK_KHR_portability_enumeration",
];

/// Vulkan instance wrapper
pub struct VulkanInstance {
    /// Ash entry point
    entry: Entry,
    /// Vulkan instance handle
    instance: Instance,
    /// Debug messenger (if validation enabled)
    debug_messenger: Option<vk::DebugUtilsMessengerEXT>,
    /// Debug utils extension loader
    debug_utils: Option<ash::ext::debug_utils::Instance>,
}

impl VulkanInstance {
    /// Create a new Vulkan instance
    pub fn new(config: &VulkanConfig) -> Result<Self, VulkanError> {
        // Load Vulkan library
        let entry = unsafe {
            Entry::load().map_err(|e| VulkanError::InstanceCreationFailed(format!("Failed to load Vulkan: {}", e)))?
        };
        
        // Check validation layer support
        let validation_enabled = config.validation_enabled && Self::check_validation_support(&entry);
        
        // Application info
        let app_name = CString::new(config.app_name.as_str()).unwrap();
        let engine_name = CString::new("Aether Engine").unwrap();
        
        let app_info = vk::ApplicationInfo::default()
            .application_name(&app_name)
            .application_version(config.app_version)
            .engine_name(&engine_name)
            .engine_version(vk::make_api_version(0, 1, 0, 0))
            .api_version(vk::API_VERSION_1_3);
        
        // Collect extensions
        let mut extensions: Vec<*const c_char> = REQUIRED_EXTENSIONS
            .iter()
            .map(|e| CString::new(*e).unwrap().into_raw() as *const c_char)
            .collect();
        
        if validation_enabled {
            extensions.push(ash::ext::debug_utils::NAME.as_ptr());
        }
        
        // Collect layers
        let layers: Vec<CString> = if validation_enabled {
            VALIDATION_LAYERS.iter().map(|l| CString::new(*l).unwrap()).collect()
        } else {
            Vec::new()
        };
        let layer_ptrs: Vec<*const c_char> = layers.iter().map(|l| l.as_ptr()).collect();
        
        // Create instance
        let create_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_extension_names(&extensions)
            .enabled_layer_names(&layer_ptrs);
        
        let instance = unsafe {
            entry.create_instance(&create_info, None)
                .map_err(|e| VulkanError::InstanceCreationFailed(format!("vkCreateInstance failed: {:?}", e)))?
        };
        
        // Setup debug messenger
        let (debug_utils, debug_messenger) = if validation_enabled {
            let debug_utils = ash::ext::debug_utils::Instance::new(&entry, &instance);
            
            let messenger_info = vk::DebugUtilsMessengerCreateInfoEXT::default()
                .message_severity(
                    vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                        | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                        | vk::DebugUtilsMessageSeverityFlagsEXT::INFO,
                )
                .message_type(
                    vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                        | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                        | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
                )
                .pfn_user_callback(Some(vulkan_debug_callback));
            
            let messenger = unsafe {
                debug_utils.create_debug_utils_messenger(&messenger_info, None)
                    .map_err(|e| VulkanError::InstanceCreationFailed(format!("Failed to create debug messenger: {:?}", e)))?
            };
            
            (Some(debug_utils), Some(messenger))
        } else {
            (None, None)
        };
        
        // Free extension strings
        for ext in extensions {
            unsafe {
                if !ext.is_null() {
                    let _ = CString::from_raw(ext as *mut c_char);
                }
            }
        }
        
        Ok(Self {
            entry,
            instance,
            debug_messenger,
            debug_utils,
        })
    }
    
    /// Check if validation layers are supported
    fn check_validation_support(entry: &Entry) -> bool {
        let available = match unsafe { entry.enumerate_instance_layer_properties() } {
            Ok(layers) => layers,
            Err(_) => return false,
        };
        
        for required in VALIDATION_LAYERS {
            let found = available.iter().any(|layer| {
                let name = unsafe { CStr::from_ptr(layer.layer_name.as_ptr()) };
                name.to_str().map(|s| s == *required).unwrap_or(false)
            });
            
            if !found {
                return false;
            }
        }
        
        true
    }
    
    /// Get the Vulkan entry point
    pub fn entry(&self) -> &Entry {
        &self.entry
    }
    
    /// Get the Vulkan instance handle
    pub fn handle(&self) -> &Instance {
        &self.instance
    }
    
    /// Get raw instance handle
    pub fn raw(&self) -> vk::Instance {
        self.instance.handle()
    }
    
    /// Enumerate physical devices
    pub fn enumerate_physical_devices(&self) -> Result<Vec<vk::PhysicalDevice>, VulkanError> {
        unsafe {
            self.instance.enumerate_physical_devices()
                .map_err(|e| VulkanError::VkError(format!("Failed to enumerate physical devices: {:?}", e)))
        }
    }
    
    /// Get physical device properties
    pub fn get_physical_device_properties(&self, device: vk::PhysicalDevice) -> vk::PhysicalDeviceProperties {
        unsafe {
            self.instance.get_physical_device_properties(device)
        }
    }
    
    /// Get physical device features
    pub fn get_physical_device_features(&self, device: vk::PhysicalDevice) -> vk::PhysicalDeviceFeatures {
        unsafe {
            self.instance.get_physical_device_features(device)
        }
    }
    
    /// Get physical device queue family properties
    pub fn get_physical_device_queue_family_properties(&self, device: vk::PhysicalDevice) -> Vec<vk::QueueFamilyProperties> {
        unsafe {
            self.instance.get_physical_device_queue_family_properties(device)
        }
    }
    
    /// Get physical device memory properties
    pub fn get_physical_device_memory_properties(&self, device: vk::PhysicalDevice) -> vk::PhysicalDeviceMemoryProperties {
        unsafe {
            self.instance.get_physical_device_memory_properties(device)
        }
    }
    
    /// Enumerate device extension properties
    pub fn enumerate_device_extension_properties(&self, device: vk::PhysicalDevice) -> Result<Vec<vk::ExtensionProperties>, VulkanError> {
        unsafe {
            self.instance.enumerate_device_extension_properties(device)
                .map_err(|e| VulkanError::VkError(format!("Failed to enumerate device extensions: {:?}", e)))
        }
    }
}

impl Drop for VulkanInstance {
    fn drop(&mut self) {
        unsafe {
            if let (Some(debug_utils), Some(messenger)) = (&self.debug_utils, self.debug_messenger) {
                debug_utils.destroy_debug_utils_messenger(messenger, None);
            }
            self.instance.destroy_instance(None);
        }
    }
}

/// Vulkan debug callback
unsafe extern "system" fn vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _p_user_data: *mut std::ffi::c_void,
) -> vk::Bool32 {
    let callback_data = *p_callback_data;
    let message = if callback_data.p_message.is_null() {
        std::borrow::Cow::from("")
    } else {
        CStr::from_ptr(callback_data.p_message).to_string_lossy()
    };
    
    let type_str = match message_type {
        vk::DebugUtilsMessageTypeFlagsEXT::GENERAL => "GENERAL",
        vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION => "VALIDATION",
        vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE => "PERFORMANCE",
        _ => "UNKNOWN",
    };
    
    match message_severity {
        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => {
            log::error!("[Vulkan {}] {}", type_str, message);
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => {
            log::warn!("[Vulkan {}] {}", type_str, message);
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::INFO => {
            log::info!("[Vulkan {}] {}", type_str, message);
        }
        _ => {
            log::debug!("[Vulkan {}] {}", type_str, message);
        }
    }
    
    vk::FALSE
}
