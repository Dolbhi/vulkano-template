//! Convientient collection of default allocators
//! Reusable for multiple renderers

use std::sync::Arc;

use vulkano::command_buffer::allocator::{CommandBufferAllocator, StandardCommandBufferAllocator};
use vulkano::descriptor_set::allocator::{DescriptorSetAllocator, StandardDescriptorSetAllocator};
use vulkano::device::Device;
use vulkano::memory::allocator::StandardMemoryAllocator;

pub struct Allocators {
    pub memory: Arc<StandardMemoryAllocator>,
    pub command_buffer: Arc<dyn CommandBufferAllocator>,
    pub descriptor_set: Arc<dyn DescriptorSetAllocator>,
}

impl Allocators {
    pub fn new(device: Arc<Device>) -> Self {
        Allocators {
            memory: Arc::new(StandardMemoryAllocator::new_default(device.clone())),
            command_buffer: Arc::new(StandardCommandBufferAllocator::new(
                device.clone(),
                Default::default(),
            )),
            descriptor_set: Arc::new(StandardDescriptorSetAllocator::new(
                device,
                Default::default(),
            )),
        }
    }
}
