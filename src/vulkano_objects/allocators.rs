//! Convientient collection of default allocators
//! Reusable for multiple renderers

use std::sync::Arc;

use vulkano::command_buffer::allocator::StandardCommandBufferAllocator;
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::device::Device;
use vulkano::memory::allocator::StandardMemoryAllocator;

pub struct Allocators {
    pub memory: Arc<StandardMemoryAllocator>,
    pub command_buffer: StandardCommandBufferAllocator,
    pub descriptor_set: StandardDescriptorSetAllocator,
}

impl Allocators {
    pub fn new(device: Arc<Device>) -> Self {
        Allocators {
            memory: Arc::new(StandardMemoryAllocator::new_default(device.clone())),
            command_buffer: StandardCommandBufferAllocator::new(device.clone(), Default::default()),
            descriptor_set: StandardDescriptorSetAllocator::new(device, Default::default()),
        }
    }
}
