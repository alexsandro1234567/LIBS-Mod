//! # Vulkan Device
//! 
//! Physical and logical device management.

use std::ffi::{CStr, CString};
use std::sync::Arc;
use ash::{vk, Device};

use super::{VulkanConfig, VulkanError, VulkanInstance};

/// Required device extensions
const REQUIRED_DEVICE_EXTENSIONS: &[&str] = &[
    "VK_KHR_swapchain",
];

/// Optional device extensions for mesh shaders
const MESH_SHADER_EXTENSIONS: &[&str] = &[
    "VK_EXT_mesh_shader",
];

/// Optional device extensions for ray tracing
const RAY_TRACING_EXTENSIONS: &[&str] = &[
    "VK_KHR_acceleration_structure",
    "VK_KHR_ray_tracing_pipeline",
    "VK_KHR_deferred_host_operations",
];

/// Queue family indices
#[derive(Debug, Clone, Copy, Default)]
pub struct QueueFamilyIndices {
    pub graphics: Option<u32>,
    pub compute: Option<u32>,
    pub transfer: Option<u32>,
    pub present: Option<u32>,
}

impl QueueFamilyIndices {
    pub fn is_complete(&self) -> bool {
        self.graphics.is_some() && self.present.is_some()
    }
}

/// Vulkan device wrapper
pub struct VulkanDevice {
    /// Instance reference
    instance: Arc<VulkanInstance>,
    /// Physical device
    physical_device: vk::PhysicalDevice,
    /// Logical device
    device: Device,
    /// Queue family indices
    queue_families: QueueFamilyIndices,
    /// Graphics queue
    graphics_queue: vk::Queue,
    /// Present queue
    present_queue: vk::Queue,
    /// Compute queue (may be same as graphics)
    compute_queue: vk::Queue,
    /// Transfer queue (may be same as graphics)
    transfer_queue: vk::Queue,
    /// Device properties
    properties: vk::PhysicalDeviceProperties,
    /// Device features
    features: vk::PhysicalDeviceFeatures,
    /// Memory properties
    memory_properties: vk::PhysicalDeviceMemoryProperties,
    /// Mesh shader support
    mesh_shaders_supported: bool,
    /// Ray tracing support
    ray_tracing_supported: bool,
}

impl VulkanDevice {
    /// Create a new Vulkan device
    pub fn new(instance: Arc<VulkanInstance>, config: &VulkanConfig) -> Result<Self, VulkanError> {
        // Enumerate physical devices
        let physical_devices = instance.enumerate_physical_devices()?;
        
        if physical_devices.is_empty() {
            return Err(VulkanError::NoSuitableGpu);
        }
        
        // Select best physical device
        let (physical_device, queue_families, mesh_supported, rt_supported) = 
            Self::select_physical_device(&instance, &physical_devices, config)?;
        
        // Get device properties
        let properties = instance.get_physical_device_properties(physical_device);
        let features = instance.get_physical_device_features(physical_device);
        let memory_properties = instance.get_physical_device_memory_properties(physical_device);
        
        // Build extension list
        let mut extensions: Vec<CString> = REQUIRED_DEVICE_EXTENSIONS
            .iter()
            .map(|e| CString::new(*e).unwrap())
            .collect();
        
        if mesh_supported && config.mesh_shaders_enabled {
            for ext in MESH_SHADER_EXTENSIONS {
                extensions.push(CString::new(*ext).unwrap());
            }
        }
        
        if rt_supported && config.ray_tracing_enabled {
            for ext in RAY_TRACING_EXTENSIONS {
                extensions.push(CString::new(*ext).unwrap());
            }
        }
        
        let extension_ptrs: Vec<*const i8> = extensions.iter().map(|e| e.as_ptr()).collect();
        
        // Create queue create infos
        let mut unique_families = vec![queue_families.graphics.unwrap()];
        if let Some(present) = queue_families.present {
            if !unique_families.contains(&present) {
                unique_families.push(present);
            }
        }
        if let Some(compute) = queue_families.compute {
            if !unique_families.contains(&compute) {
                unique_families.push(compute);
            }
        }
        if let Some(transfer) = queue_families.transfer {
            if !unique_families.contains(&transfer) {
                unique_families.push(transfer);
            }
        }
        
        let queue_priorities = [1.0f32];
        let queue_create_infos: Vec<vk::DeviceQueueCreateInfo> = unique_families
            .iter()
            .map(|&family| {
                vk::DeviceQueueCreateInfo::default()
                    .queue_family_index(family)
                    .queue_priorities(&queue_priorities)
            })
            .collect();
        
        // Device features
        let device_features = vk::PhysicalDeviceFeatures::default()
            .sampler_anisotropy(true)
            .fill_mode_non_solid(true)
            .wide_lines(true)
            .multi_draw_indirect(true);
        
        // Vulkan 1.2 features
        let mut vulkan12_features = vk::PhysicalDeviceVulkan12Features::default()
            .buffer_device_address(true)
            .descriptor_indexing(true)
            .runtime_descriptor_array(true)
            .descriptor_binding_partially_bound(true)
            .descriptor_binding_variable_descriptor_count(true)
            .shader_sampled_image_array_non_uniform_indexing(true);
        
        // Vulkan 1.3 features
        let mut vulkan13_features = vk::PhysicalDeviceVulkan13Features::default()
            .dynamic_rendering(true)
            .synchronization2(true)
            .maintenance4(true);
        
        // Mesh shader features
        let mut mesh_shader_features = vk::PhysicalDeviceMeshShaderFeaturesEXT::default()
            .mesh_shader(mesh_supported && config.mesh_shaders_enabled)
            .task_shader(mesh_supported && config.mesh_shaders_enabled);
        
        // Chain features
        let mut create_info = vk::DeviceCreateInfo::default()
            .queue_create_infos(&queue_create_infos)
            .enabled_extension_names(&extension_ptrs)
            .enabled_features(&device_features)
            .push_next(&mut vulkan12_features)
            .push_next(&mut vulkan13_features);
        
        if mesh_supported && config.mesh_shaders_enabled {
            create_info = create_info.push_next(&mut mesh_shader_features);
        }
        
        // Create logical device
        let device = unsafe {
            instance.handle().create_device(physical_device, &create_info, None)
                .map_err(|e| VulkanError::DeviceCreationFailed(format!("vkCreateDevice failed: {:?}", e)))?
        };
        
        // Get queues
        let graphics_queue = unsafe { device.get_device_queue(queue_families.graphics.unwrap(), 0) };
        let present_queue = unsafe { device.get_device_queue(queue_families.present.unwrap_or(queue_families.graphics.unwrap()), 0) };
        let compute_queue = unsafe { device.get_device_queue(queue_families.compute.unwrap_or(queue_families.graphics.unwrap()), 0) };
        let transfer_queue = unsafe { device.get_device_queue(queue_families.transfer.unwrap_or(queue_families.graphics.unwrap()), 0) };
        
        Ok(Self {
            instance,
            physical_device,
            device,
            queue_families,
            graphics_queue,
            present_queue,
            compute_queue,
            transfer_queue,
            properties,
            features,
            memory_properties,
            mesh_shaders_supported: mesh_supported && config.mesh_shaders_enabled,
            ray_tracing_supported: rt_supported && config.ray_tracing_enabled,
        })
    }
    
    /// Select the best physical device
    fn select_physical_device(
        instance: &VulkanInstance,
        devices: &[vk::PhysicalDevice],
        config: &VulkanConfig,
    ) -> Result<(vk::PhysicalDevice, QueueFamilyIndices, bool, bool), VulkanError> {
        let mut best_device = None;
        let mut best_score = 0;
        
        for &device in devices {
            let properties = instance.get_physical_device_properties(device);
            let features = instance.get_physical_device_features(device);
            let queue_families = Self::find_queue_families(instance, device);
            
            // Check required extensions
            let extensions = instance.enumerate_device_extension_properties(device)?;
            let has_required = REQUIRED_DEVICE_EXTENSIONS.iter().all(|required| {
                extensions.iter().any(|ext| {
                    let name = unsafe { CStr::from_ptr(ext.extension_name.as_ptr()) };
                    name.to_str().map(|s| s == *required).unwrap_or(false)
                })
            });
            
            if !has_required || !queue_families.is_complete() {
                continue;
            }
            
            // Check optional extensions
            let has_mesh_shaders = config.mesh_shaders_enabled && MESH_SHADER_EXTENSIONS.iter().all(|required| {
                extensions.iter().any(|ext| {
                    let name = unsafe { CStr::from_ptr(ext.extension_name.as_ptr()) };
                    name.to_str().map(|s| s == *required).unwrap_or(false)
                })
            });
            
            let has_ray_tracing = config.ray_tracing_enabled && RAY_TRACING_EXTENSIONS.iter().all(|required| {
                extensions.iter().any(|ext| {
                    let name = unsafe { CStr::from_ptr(ext.extension_name.as_ptr()) };
                    name.to_str().map(|s| s == *required).unwrap_or(false)
                })
            });
            
            // Score the device
            let mut score = 0;
            
            // Prefer discrete GPUs
            if properties.device_type == vk::PhysicalDeviceType::DISCRETE_GPU {
                score += 10000;
            } else if properties.device_type == vk::PhysicalDeviceType::INTEGRATED_GPU {
                score += 1000;
            }
            
            // Add VRAM size
            score += (properties.limits.max_image_dimension2_d / 1024) as i32;
            
            // Bonus for features
            if features.geometry_shader == vk::TRUE {
                score += 100;
            }
            if features.tessellation_shader == vk::TRUE {
                score += 100;
            }
            if has_mesh_shaders {
                score += 500;
            }
            if has_ray_tracing {
                score += 500;
            }
            
            if score > best_score {
                best_score = score;
                best_device = Some((device, queue_families, has_mesh_shaders, has_ray_tracing));
            }
        }
        
        best_device.ok_or(VulkanError::NoSuitableGpu)
    }
    
    /// Find queue family indices
    fn find_queue_families(instance: &VulkanInstance, device: vk::PhysicalDevice) -> QueueFamilyIndices {
        let queue_families = instance.get_physical_device_queue_family_properties(device);
        
        let mut indices = QueueFamilyIndices::default();
        
        for (i, family) in queue_families.iter().enumerate() {
            let i = i as u32;
            
            // Graphics queue
            if family.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                indices.graphics = Some(i);
                indices.present = Some(i); // Assume same for now
            }
            
            // Dedicated compute queue
            if family.queue_flags.contains(vk::QueueFlags::COMPUTE) 
                && !family.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                indices.compute = Some(i);
            }
            
            // Dedicated transfer queue
            if family.queue_flags.contains(vk::QueueFlags::TRANSFER)
                && !family.queue_flags.contains(vk::QueueFlags::GRAPHICS)
                && !family.queue_flags.contains(vk::QueueFlags::COMPUTE) {
                indices.transfer = Some(i);
            }
        }
        
        // Fallback compute to graphics
        if indices.compute.is_none() {
            indices.compute = indices.graphics;
        }
        
        // Fallback transfer to graphics
        if indices.transfer.is_none() {
            indices.transfer = indices.graphics;
        }
        
        indices
    }
    
    /// Wait for device idle
    pub fn wait_idle(&self) -> Result<(), VulkanError> {
        unsafe {
            self.device.device_wait_idle()
                .map_err(|e| VulkanError::VkError(format!("vkDeviceWaitIdle failed: {:?}", e)))
        }
    }
    
    /// Get GPU name
    pub fn gpu_name(&self) -> String {
        let name = unsafe { CStr::from_ptr(self.properties.device_name.as_ptr()) };
        name.to_string_lossy().into_owned()
    }
    
    /// Check mesh shader support
    pub fn supports_mesh_shaders(&self) -> bool {
        self.mesh_shaders_supported
    }
    
    /// Check ray tracing support
    pub fn supports_ray_tracing(&self) -> bool {
        self.ray_tracing_supported
    }
    
    /// Get logical device handle
    pub fn handle(&self) -> &Device {
        &self.device
    }
    
    /// Get raw device handle
    pub fn raw(&self) -> vk::Device {
        self.device.handle()
    }
    
    /// Get physical device
    pub fn physical_device(&self) -> vk::PhysicalDevice {
        self.physical_device
    }
    
    /// Get queue family indices
    pub fn queue_families(&self) -> &QueueFamilyIndices {
        &self.queue_families
    }
    
    /// Get graphics queue
    pub fn graphics_queue(&self) -> vk::Queue {
        self.graphics_queue
    }
    
    /// Get present queue
    pub fn present_queue(&self) -> vk::Queue {
        self.present_queue
    }
    
    /// Get compute queue
    pub fn compute_queue(&self) -> vk::Queue {
        self.compute_queue
    }
    
    /// Get transfer queue
    pub fn transfer_queue(&self) -> vk::Queue {
        self.transfer_queue
    }
    
    /// Get memory properties
    pub fn memory_properties(&self) -> &vk::PhysicalDeviceMemoryProperties {
        &self.memory_properties
    }
    
    /// Get device properties
    pub fn properties(&self) -> &vk::PhysicalDeviceProperties {
        &self.properties
    }
    
    /// Get instance reference
    pub fn instance(&self) -> &Arc<VulkanInstance> {
        &self.instance
    }
    
    /// Find memory type index
    pub fn find_memory_type(&self, type_filter: u32, properties: vk::MemoryPropertyFlags) -> Option<u32> {
        for i in 0..self.memory_properties.memory_type_count {
            if (type_filter & (1 << i)) != 0 
                && self.memory_properties.memory_types[i as usize].property_flags.contains(properties) {
                return Some(i);
            }
        }
        None
    }
}

impl Drop for VulkanDevice {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_device(None);
        }
    }
}
