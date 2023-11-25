use std::sync::Arc;

use vulkano::device::Device;
use vulkano::format::Format;
use vulkano::image::view::ImageView;
use vulkano::image::{Image, ImageCreateInfo, ImageType, ImageUsage};
use vulkano::render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass};
use vulkano::swapchain::Swapchain;

use super::allocators::Allocators;

pub fn create_render_pass(device: Arc<Device>, swapchain: Arc<Swapchain>) -> Arc<RenderPass> {
    vulkano::single_pass_renderpass!(
        device,
        attachments: {
            color: {
                format: swapchain.image_format(),
                samples: 1,
                load_op: Clear,
                store_op: Store,
            },
            depth_stencil: {
                format: Format::D32_SFLOAT,
                samples: 1,
                load_op: Clear,
                store_op: DontCare,
            }
        },
        pass: {
            color: [color],
            depth_stencil: {depth_stencil},
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
