use std::sync::Arc;

use vulkano::device::physical::PhysicalDevice;
use vulkano::device::Device;
use vulkano::image::{Image, ImageUsage};
use vulkano::swapchain::{Surface, Swapchain, SwapchainCreateInfo};
use winit::window::Window;

/// create swapchain and swapchain images appropriate for given device and surface
pub fn create_swapchain(
    physical_device: &Arc<PhysicalDevice>,
    device: Arc<Device>,
    surface: Arc<Surface>,
) -> (Arc<Swapchain>, Vec<Arc<Image>>) {
    let caps = physical_device
        .surface_capabilities(&surface, Default::default())
        .expect("failed to get surface capabilities");

    let composite_alpha = caps.supported_composite_alpha.into_iter().next().unwrap();
    let image_format = physical_device
        .surface_formats(&surface, Default::default())
        .unwrap()[0]
        .0;

    println!("Surface formats: {:?}", {
        physical_device
            .surface_formats(&surface, Default::default())
            .unwrap()
    });

    Swapchain::new(
        device,
        surface.clone(),
        SwapchainCreateInfo {
            min_image_count: caps.min_image_count.max(2),
            image_format,
            image_extent: surface
                .object()
                .unwrap()
                .clone()
                .downcast::<Window>()
                .unwrap()
                .inner_size()
                .into(),
            image_usage: ImageUsage::COLOR_ATTACHMENT,
            composite_alpha,
            ..Default::default()
        },
    )
    .unwrap()
}
