//! Single function for comparing and selecting an appropriate device
//! Reusable for multiple renderers

use std::sync::Arc;

use vulkano::device::physical::{PhysicalDevice, PhysicalDeviceType};
use vulkano::device::{DeviceExtensions, Features, QueueFlags};
use vulkano::instance::Instance;
use vulkano::swapchain::Surface;

pub fn select_physical_device(
    instance: &Arc<Instance>,
    surface: Arc<Surface>,
    device_extensions: &DeviceExtensions,
    device_features: &Features,
) -> (Arc<PhysicalDevice>, u32) {
    instance
        .enumerate_physical_devices()
        .expect("failed to enumerate physical devices")
        .filter(|p| p.supported_extensions().contains(device_extensions))
        .filter(|p| p.supported_features().contains(device_features))
        .filter_map(|p| {
            p.queue_family_properties()
                .iter()
                .enumerate()
                .position(|(i, q)| {
                    q.queue_flags.contains(QueueFlags::GRAPHICS)
                        && p.surface_support(i as u32, &surface).unwrap_or(false)
                })
                .map(|q| (p, q as u32))
        })
        .min_by_key(|(p, _)| match p.properties().device_type {
            PhysicalDeviceType::DiscreteGpu => 0,
            PhysicalDeviceType::IntegratedGpu => 1,
            PhysicalDeviceType::VirtualGpu => 2,
            PhysicalDeviceType::Cpu => 3,
            _ => 4,
        })
        .expect("no device available")
}
