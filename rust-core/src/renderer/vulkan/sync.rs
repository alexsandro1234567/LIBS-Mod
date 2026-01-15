//! # Vulkan Synchronization
//! 
//! Semaphores and fences for GPU synchronization.

use std::sync::Arc;
use ash::vk;

use super::{VulkanDevice, VulkanError};

/// Synchronization objects for frame rendering
pub struct SyncObjects {
    /// Device reference
    device: Arc<VulkanDevice>,
    /// Image available semaphores (one per frame in flight)
    image_available: Vec<vk::Semaphore>,
    /// Render finished semaphores (one per frame in flight)
    render_finished: Vec<vk::Semaphore>,
    /// In-flight fences (one per frame in flight)
    in_flight_fences: Vec<vk::Fence>,
    /// Number of frames in flight
    frames_in_flight: usize,
}

impl SyncObjects {
    /// Create synchronization objects
    pub fn new(device: Arc<VulkanDevice>, frames_in_flight: usize) -> Result<Self, VulkanError> {
        let semaphore_info = vk::SemaphoreCreateInfo::default();
        let fence_info = vk::FenceCreateInfo::default()
            .flags(vk::FenceCreateFlags::SIGNALED); // Start signaled so first wait doesn't block
        
        let mut image_available = Vec::with_capacity(frames_in_flight);
        let mut render_finished = Vec::with_capacity(frames_in_flight);
        let mut in_flight_fences = Vec::with_capacity(frames_in_flight);
        
        for _ in 0..frames_in_flight {
            let image_sem = unsafe {
                device.handle().create_semaphore(&semaphore_info, None)
                    .map_err(|e| VulkanError::SyncError(format!("Failed to create semaphore: {:?}", e)))?
            };
            
            let render_sem = unsafe {
                device.handle().create_semaphore(&semaphore_info, None)
                    .map_err(|e| VulkanError::SyncError(format!("Failed to create semaphore: {:?}", e)))?
            };
            
            let fence = unsafe {
                device.handle().create_fence(&fence_info, None)
                    .map_err(|e| VulkanError::SyncError(format!("Failed to create fence: {:?}", e)))?
            };
            
            image_available.push(image_sem);
            render_finished.push(render_sem);
            in_flight_fences.push(fence);
        }
        
        Ok(Self {
            device,
            image_available,
            render_finished,
            in_flight_fences,
            frames_in_flight,
        })
    }
    
    /// Wait for fence at frame index
    pub fn wait_for_fence(&self, frame_index: usize) -> Result<(), VulkanError> {
        let fence = self.in_flight_fences[frame_index];
        
        unsafe {
            self.device.handle().wait_for_fences(&[fence], true, u64::MAX)
                .map_err(|e| VulkanError::SyncError(format!("Failed to wait for fence: {:?}", e)))
        }
    }
    
    /// Reset fence at frame index
    pub fn reset_fence(&self, frame_index: usize) -> Result<(), VulkanError> {
        let fence = self.in_flight_fences[frame_index];
        
        unsafe {
            self.device.handle().reset_fences(&[fence])
                .map_err(|e| VulkanError::SyncError(format!("Failed to reset fence: {:?}", e)))
        }
    }
    
    /// Get image available semaphore
    pub fn image_available(&self, frame_index: usize) -> vk::Semaphore {
        self.image_available[frame_index]
    }
    
    /// Get render finished semaphore
    pub fn render_finished(&self, frame_index: usize) -> vk::Semaphore {
        self.render_finished[frame_index]
    }
    
    /// Get in-flight fence
    pub fn in_flight_fence(&self, frame_index: usize) -> vk::Fence {
        self.in_flight_fences[frame_index]
    }
    
    /// Get frames in flight count
    pub fn frames_in_flight(&self) -> usize {
        self.frames_in_flight
    }
}

impl Drop for SyncObjects {
    fn drop(&mut self) {
        unsafe {
            for &sem in &self.image_available {
                self.device.handle().destroy_semaphore(sem, None);
            }
            for &sem in &self.render_finished {
                self.device.handle().destroy_semaphore(sem, None);
            }
            for &fence in &self.in_flight_fences {
                self.device.handle().destroy_fence(fence, None);
            }
        }
    }
}

/// Timeline semaphore for advanced synchronization
pub struct TimelineSemaphore {
    device: Arc<VulkanDevice>,
    semaphore: vk::Semaphore,
    current_value: u64,
}

impl TimelineSemaphore {
    /// Create a new timeline semaphore
    pub fn new(device: Arc<VulkanDevice>, initial_value: u64) -> Result<Self, VulkanError> {
        let mut type_info = vk::SemaphoreTypeCreateInfo::default()
            .semaphore_type(vk::SemaphoreType::TIMELINE)
            .initial_value(initial_value);
        
        let semaphore_info = vk::SemaphoreCreateInfo::default()
            .push_next(&mut type_info);
        
        let semaphore = unsafe {
            device.handle().create_semaphore(&semaphore_info, None)
                .map_err(|e| VulkanError::SyncError(format!("Failed to create timeline semaphore: {:?}", e)))?
        };
        
        Ok(Self {
            device,
            semaphore,
            current_value: initial_value,
        })
    }
    
    /// Signal the semaphore with a new value
    pub fn signal(&mut self, value: u64) -> Result<(), VulkanError> {
        let signal_info = vk::SemaphoreSignalInfo::default()
            .semaphore(self.semaphore)
            .value(value);
        
        unsafe {
            self.device.handle().signal_semaphore(&signal_info)
                .map_err(|e| VulkanError::SyncError(format!("Failed to signal semaphore: {:?}", e)))?;
        }
        
        self.current_value = value;
        Ok(())
    }
    
    /// Wait for the semaphore to reach a value
    pub fn wait(&self, value: u64, timeout: u64) -> Result<(), VulkanError> {
        let semaphores = [self.semaphore];
        let values = [value];
        
        let wait_info = vk::SemaphoreWaitInfo::default()
            .semaphores(&semaphores)
            .values(&values);
        
        unsafe {
            self.device.handle().wait_semaphores(&wait_info, timeout)
                .map_err(|e| VulkanError::SyncError(format!("Failed to wait for semaphore: {:?}", e)))
        }
    }
    
    /// Get current counter value
    pub fn get_value(&self) -> Result<u64, VulkanError> {
        unsafe {
            self.device.handle().get_semaphore_counter_value(self.semaphore)
                .map_err(|e| VulkanError::SyncError(format!("Failed to get semaphore value: {:?}", e)))
        }
    }
    
    /// Get semaphore handle
    pub fn handle(&self) -> vk::Semaphore {
        self.semaphore
    }
    
    /// Get current tracked value
    pub fn current_value(&self) -> u64 {
        self.current_value
    }
}

impl Drop for TimelineSemaphore {
    fn drop(&mut self) {
        unsafe {
            self.device.handle().destroy_semaphore(self.semaphore, None);
        }
    }
}
