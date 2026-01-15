//! # Vulkan Swapchain
//! 
//! Swapchain creation and management.

use std::sync::Arc;
use ash::{vk, khr};

use super::{VulkanConfig, VulkanDevice, VulkanError, VulkanInstance};

/// Swapchain wrapper
pub struct Swapchain {
    /// Instance reference
    instance: Arc<VulkanInstance>,
    /// Device reference
    device: Arc<VulkanDevice>,
    /// Swapchain extension loader
    swapchain_loader: khr::swapchain::Device,
    /// Swapchain handle
    swapchain: vk::SwapchainKHR,
    /// Surface handle
    surface: vk::SurfaceKHR,
    /// Surface loader
    surface_loader: khr::surface::Instance,
    /// Swapchain images
    images: Vec<vk::Image>,
    /// Swapchain image views
    image_views: Vec<vk::ImageView>,
    /// Swapchain format
    format: vk::SurfaceFormatKHR,
    /// Swapchain extent
    extent: vk::Extent2D,
    /// Present mode
    present_mode: vk::PresentModeKHR,
    /// Depth image
    depth_image: vk::Image,
    /// Depth image memory
    depth_memory: vk::DeviceMemory,
    /// Depth image view
    depth_view: vk::ImageView,
    /// Depth format
    depth_format: vk::Format,
}

impl Swapchain {
    /// Create a new swapchain
    pub fn new(
        instance: Arc<VulkanInstance>,
        device: Arc<VulkanDevice>,
        window_handle: u64,
        width: u32,
        height: u32,
        config: &VulkanConfig,
    ) -> Result<Self, VulkanError> {
        // Create surface (platform-specific)
        let surface_loader = khr::surface::Instance::new(instance.entry(), instance.handle());
        let surface = Self::create_surface(&instance, window_handle)?;
        
        // Create swapchain loader
        let swapchain_loader = khr::swapchain::Device::new(instance.handle(), device.handle());
        
        // Query surface capabilities
        let capabilities = unsafe {
            surface_loader.get_physical_device_surface_capabilities(device.physical_device(), surface)
                .map_err(|e| VulkanError::SwapchainCreationFailed(format!("Failed to get surface capabilities: {:?}", e)))?
        };
        
        // Choose surface format
        let formats = unsafe {
            surface_loader.get_physical_device_surface_formats(device.physical_device(), surface)
                .map_err(|e| VulkanError::SwapchainCreationFailed(format!("Failed to get surface formats: {:?}", e)))?
        };
        
        let format = Self::choose_surface_format(&formats);
        
        // Choose present mode
        let present_modes = unsafe {
            surface_loader.get_physical_device_surface_present_modes(device.physical_device(), surface)
                .map_err(|e| VulkanError::SwapchainCreationFailed(format!("Failed to get present modes: {:?}", e)))?
        };
        
        let present_mode = Self::choose_present_mode(&present_modes, config.preferred_present_mode);
        
        // Choose extent
        let extent = Self::choose_extent(&capabilities, width, height);
        
        // Choose image count
        let mut image_count = capabilities.min_image_count + 1;
        if capabilities.max_image_count > 0 && image_count > capabilities.max_image_count {
            image_count = capabilities.max_image_count;
        }
        
        // Create swapchain
        let create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(surface)
            .min_image_count(image_count)
            .image_format(format.format)
            .image_color_space(format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_DST)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true);
        
        let swapchain = unsafe {
            swapchain_loader.create_swapchain(&create_info, None)
                .map_err(|e| VulkanError::SwapchainCreationFailed(format!("vkCreateSwapchainKHR failed: {:?}", e)))?
        };
        
        // Get swapchain images
        let images = unsafe {
            swapchain_loader.get_swapchain_images(swapchain)
                .map_err(|e| VulkanError::SwapchainCreationFailed(format!("Failed to get swapchain images: {:?}", e)))?
        };
        
        // Create image views
        let image_views = Self::create_image_views(&device, &images, format.format)?;
        
        // Create depth resources
        let depth_format = Self::find_depth_format(&device)?;
        let (depth_image, depth_memory, depth_view) = Self::create_depth_resources(&device, extent, depth_format)?;
        
        Ok(Self {
            instance,
            device,
            swapchain_loader,
            swapchain,
            surface,
            surface_loader,
            images,
            image_views,
            format,
            extent,
            present_mode,
            depth_image,
            depth_memory,
            depth_view,
            depth_format,
        })
    }
    
    /// Create platform-specific surface
    fn create_surface(instance: &VulkanInstance, window_handle: u64) -> Result<vk::SurfaceKHR, VulkanError> {
        // This is a simplified version - in production would use raw-window-handle
        // For now, create a headless surface or return a dummy
        
        #[cfg(target_os = "windows")]
        {
            use ash::khr::win32_surface;
            
            let win32_loader = win32_surface::Instance::new(instance.entry(), instance.handle());
            let create_info = vk::Win32SurfaceCreateInfoKHR::default()
                .hinstance(0 as isize)
                .hwnd(window_handle as isize);
            
            unsafe {
                win32_loader.create_win32_surface(&create_info, None)
                    .map_err(|e| VulkanError::SwapchainCreationFailed(format!("Failed to create Win32 surface: {:?}", e)))
            }
        }
        
        #[cfg(not(target_os = "windows"))]
        {
            // For non-Windows, would need XCB/Xlib/Wayland surface
            // Return error for now
            Err(VulkanError::SwapchainCreationFailed("Surface creation not implemented for this platform".to_string()))
        }
    }
    
    /// Choose best surface format
    fn choose_surface_format(formats: &[vk::SurfaceFormatKHR]) -> vk::SurfaceFormatKHR {
        // Prefer SRGB with B8G8R8A8
        for format in formats {
            if format.format == vk::Format::B8G8R8A8_SRGB 
                && format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR {
                return *format;
            }
        }
        
        // Fallback to first available
        formats[0]
    }
    
    /// Choose best present mode
    fn choose_present_mode(modes: &[vk::PresentModeKHR], preferred: vk::PresentModeKHR) -> vk::PresentModeKHR {
        // Try preferred first
        if modes.contains(&preferred) {
            return preferred;
        }
        
        // Try mailbox (triple buffering)
        if modes.contains(&vk::PresentModeKHR::MAILBOX) {
            return vk::PresentModeKHR::MAILBOX;
        }
        
        // Fallback to FIFO (always available)
        vk::PresentModeKHR::FIFO
    }
    
    /// Choose swapchain extent
    fn choose_extent(capabilities: &vk::SurfaceCapabilitiesKHR, width: u32, height: u32) -> vk::Extent2D {
        if capabilities.current_extent.width != u32::MAX {
            capabilities.current_extent
        } else {
            vk::Extent2D {
                width: width.clamp(capabilities.min_image_extent.width, capabilities.max_image_extent.width),
                height: height.clamp(capabilities.min_image_extent.height, capabilities.max_image_extent.height),
            }
        }
    }
    
    /// Create image views for swapchain images
    fn create_image_views(device: &VulkanDevice, images: &[vk::Image], format: vk::Format) -> Result<Vec<vk::ImageView>, VulkanError> {
        let mut views = Vec::with_capacity(images.len());
        
        for &image in images {
            let create_info = vk::ImageViewCreateInfo::default()
                .image(image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(format)
                .components(vk::ComponentMapping {
                    r: vk::ComponentSwizzle::IDENTITY,
                    g: vk::ComponentSwizzle::IDENTITY,
                    b: vk::ComponentSwizzle::IDENTITY,
                    a: vk::ComponentSwizzle::IDENTITY,
                })
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                });
            
            let view = unsafe {
                device.handle().create_image_view(&create_info, None)
                    .map_err(|e| VulkanError::SwapchainCreationFailed(format!("Failed to create image view: {:?}", e)))?
            };
            
            views.push(view);
        }
        
        Ok(views)
    }
    
    /// Find supported depth format
    fn find_depth_format(device: &VulkanDevice) -> Result<vk::Format, VulkanError> {
        let candidates = [
            vk::Format::D32_SFLOAT,
            vk::Format::D32_SFLOAT_S8_UINT,
            vk::Format::D24_UNORM_S8_UINT,
        ];
        
        for format in candidates {
            let props = unsafe {
                device.instance().handle().get_physical_device_format_properties(device.physical_device(), format)
            };
            
            if props.optimal_tiling_features.contains(vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT) {
                return Ok(format);
            }
        }
        
        Err(VulkanError::SwapchainCreationFailed("No suitable depth format found".to_string()))
    }
    
    /// Create depth buffer resources
    fn create_depth_resources(
        device: &VulkanDevice,
        extent: vk::Extent2D,
        format: vk::Format,
    ) -> Result<(vk::Image, vk::DeviceMemory, vk::ImageView), VulkanError> {
        // Create depth image
        let image_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(format)
            .extent(vk::Extent3D {
                width: extent.width,
                height: extent.height,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED);
        
        let image = unsafe {
            device.handle().create_image(&image_info, None)
                .map_err(|e| VulkanError::SwapchainCreationFailed(format!("Failed to create depth image: {:?}", e)))?
        };
        
        // Allocate memory
        let mem_requirements = unsafe { device.handle().get_image_memory_requirements(image) };
        
        let mem_type = device.find_memory_type(
            mem_requirements.memory_type_bits,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        ).ok_or_else(|| VulkanError::SwapchainCreationFailed("No suitable memory type for depth buffer".to_string()))?;
        
        let alloc_info = vk::MemoryAllocateInfo::default()
            .allocation_size(mem_requirements.size)
            .memory_type_index(mem_type);
        
        let memory = unsafe {
            device.handle().allocate_memory(&alloc_info, None)
                .map_err(|e| VulkanError::SwapchainCreationFailed(format!("Failed to allocate depth memory: {:?}", e)))?
        };
        
        // Bind memory
        unsafe {
            device.handle().bind_image_memory(image, memory, 0)
                .map_err(|e| VulkanError::SwapchainCreationFailed(format!("Failed to bind depth memory: {:?}", e)))?;
        }
        
        // Create image view
        let view_info = vk::ImageViewCreateInfo::default()
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(format)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::DEPTH,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            });
        
        let view = unsafe {
            device.handle().create_image_view(&view_info, None)
                .map_err(|e| VulkanError::SwapchainCreationFailed(format!("Failed to create depth view: {:?}", e)))?
        };
        
        Ok((image, memory, view))
    }
    
    /// Acquire next swapchain image
    pub fn acquire_next_image(&self, semaphore: vk::Semaphore) -> Result<u32, VulkanError> {
        unsafe {
            let (index, _suboptimal) = self.swapchain_loader
                .acquire_next_image(self.swapchain, u64::MAX, semaphore, vk::Fence::null())
                .map_err(|e| match e {
                    vk::Result::ERROR_OUT_OF_DATE_KHR => VulkanError::OutOfDate,
                    vk::Result::ERROR_SURFACE_LOST_KHR => VulkanError::SurfaceLost,
                    _ => VulkanError::VkError(format!("Failed to acquire image: {:?}", e)),
                })?;
            
            Ok(index)
        }
    }
    
    /// Present the current image
    pub fn present(&self, image_index: u32, wait_semaphore: vk::Semaphore) -> Result<(), VulkanError> {
        let swapchains = [self.swapchain];
        let image_indices = [image_index];
        let wait_semaphores = [wait_semaphore];
        
        let present_info = vk::PresentInfoKHR::default()
            .wait_semaphores(&wait_semaphores)
            .swapchains(&swapchains)
            .image_indices(&image_indices);
        
        unsafe {
            self.swapchain_loader.queue_present(self.device.present_queue(), &present_info)
                .map_err(|e| match e {
                    vk::Result::ERROR_OUT_OF_DATE_KHR => VulkanError::OutOfDate,
                    vk::Result::ERROR_SURFACE_LOST_KHR => VulkanError::SurfaceLost,
                    _ => VulkanError::VkError(format!("Failed to present: {:?}", e)),
                })?;
        }
        
        Ok(())
    }
    
    /// Recreate swapchain
    pub fn recreate(&mut self, width: u32, height: u32) -> Result<(), VulkanError> {
        // Wait for device idle
        self.device.wait_idle()?;
        
        // Cleanup old resources
        self.cleanup_swapchain();
        
        // Query new capabilities
        let capabilities = unsafe {
            self.surface_loader.get_physical_device_surface_capabilities(self.device.physical_device(), self.surface)
                .map_err(|e| VulkanError::SwapchainCreationFailed(format!("Failed to get surface capabilities: {:?}", e)))?
        };
        
        // Choose new extent
        self.extent = Self::choose_extent(&capabilities, width, height);
        
        // Choose image count
        let mut image_count = capabilities.min_image_count + 1;
        if capabilities.max_image_count > 0 && image_count > capabilities.max_image_count {
            image_count = capabilities.max_image_count;
        }
        
        // Create new swapchain
        let create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(self.surface)
            .min_image_count(image_count)
            .image_format(self.format.format)
            .image_color_space(self.format.color_space)
            .image_extent(self.extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_DST)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(self.present_mode)
            .clipped(true)
            .old_swapchain(self.swapchain);
        
        let new_swapchain = unsafe {
            self.swapchain_loader.create_swapchain(&create_info, None)
                .map_err(|e| VulkanError::SwapchainCreationFailed(format!("vkCreateSwapchainKHR failed: {:?}", e)))?
        };
        
        // Destroy old swapchain
        unsafe {
            self.swapchain_loader.destroy_swapchain(self.swapchain, None);
        }
        self.swapchain = new_swapchain;
        
        // Get new images
        self.images = unsafe {
            self.swapchain_loader.get_swapchain_images(self.swapchain)
                .map_err(|e| VulkanError::SwapchainCreationFailed(format!("Failed to get swapchain images: {:?}", e)))?
        };
        
        // Create new image views
        self.image_views = Self::create_image_views(&self.device, &self.images, self.format.format)?;
        
        // Recreate depth resources
        let (depth_image, depth_memory, depth_view) = Self::create_depth_resources(&self.device, self.extent, self.depth_format)?;
        self.depth_image = depth_image;
        self.depth_memory = depth_memory;
        self.depth_view = depth_view;
        
        Ok(())
    }
    
    /// Cleanup swapchain resources
    fn cleanup_swapchain(&mut self) {
        unsafe {
            // Destroy depth resources
            self.device.handle().destroy_image_view(self.depth_view, None);
            self.device.handle().destroy_image(self.depth_image, None);
            self.device.handle().free_memory(self.depth_memory, None);
            
            // Destroy image views
            for view in &self.image_views {
                self.device.handle().destroy_image_view(*view, None);
            }
            self.image_views.clear();
        }
    }
    
    /// Get swapchain format
    pub fn format(&self) -> vk::Format {
        self.format.format
    }
    
    /// Get swapchain extent
    pub fn extent(&self) -> vk::Extent2D {
        self.extent
    }
    
    /// Get image count
    pub fn image_count(&self) -> usize {
        self.images.len()
    }
    
    /// Get image view at index
    pub fn image_view(&self, index: usize) -> vk::ImageView {
        self.image_views[index]
    }
    
    /// Get depth view
    pub fn depth_view(&self) -> vk::ImageView {
        self.depth_view
    }
    
    /// Get depth format
    pub fn depth_format(&self) -> vk::Format {
        self.depth_format
    }
}

impl Drop for Swapchain {
    fn drop(&mut self) {
        unsafe {
            self.device.wait_idle().ok();
            
            self.cleanup_swapchain();
            
            self.swapchain_loader.destroy_swapchain(self.swapchain, None);
            self.surface_loader.destroy_surface(self.surface, None);
        }
    }
}
