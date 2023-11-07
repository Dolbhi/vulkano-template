use std::sync::Arc;

use vulkano::device::physical::PhysicalDevice;
use vulkano::device::Device;
use vulkano::format::Format;
use vulkano::image::view::ImageView;
use vulkano::image::{Image, ImageCreateInfo, ImageType, ImageUsage};
use vulkano::render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass};
use vulkano::swapchain::{Surface, Swapchain, SwapchainCreateInfo};

use super::allocators::Allocators;

use winit::window::Window;

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

pub fn create_framebuffers_from_swapchain_images(
    images: &[Arc<Image>],
    render_pass: Arc<RenderPass>,
    memory_allocator: &Allocators,
) -> Vec<Arc<Framebuffer>> {
    let depth_attachment = ImageView::new_default(
        Image::new(
            memory_allocator.memory.clone(),
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                format: Format::D32_SFLOAT,
                extent: images[0].extent(),
                usage: ImageUsage::DEPTH_STENCIL_ATTACHMENT | ImageUsage::TRANSIENT_ATTACHMENT,
                ..Default::default()
            },
            Default::default(),
        )
        .unwrap(),
    )
    .unwrap();

    images
        .iter()
        .map(|image| {
            let view = ImageView::new_default(image.clone()).unwrap();
            Framebuffer::new(
                render_pass.clone(),
                FramebufferCreateInfo {
                    attachments: vec![view, depth_attachment.clone()],
                    ..Default::default()
                },
            )
            .unwrap()
        })
        .collect::<Vec<_>>()
}
