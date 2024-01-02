//! Convient renderpass and framebuffer creation functions
//! VERY NOT reusable for multiple renderers

use std::sync::Arc;

use vulkano::device::Device;
use vulkano::format::Format;
use vulkano::image::view::ImageView;
use vulkano::image::{Image, ImageCreateInfo, ImageType, ImageUsage};
use vulkano::render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass};
use vulkano::swapchain::Swapchain;

use super::allocators::Allocators;

/// Creates a single pass renderpass with a depth buffer
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
        .collect()
}

pub fn create_deferred_render_pass(
    device: Arc<Device>,
    swapchain: Arc<Swapchain>,
) -> Arc<RenderPass> {
    vulkano::ordered_passes_renderpass!(
        device,
    attachments: {
            // The image that will contain the final rendering (in this example the swapchain
            // image, but it could be another image).
            final_color: {
                format: swapchain.image_format(),
                samples: 1,
                load_op: Clear,
                store_op: Store,
            },
            // Diffuse buffer (unlit color)
            diffuse: {
                format: Format::A2B10G10R10_UNORM_PACK32,
                samples: 1,
                load_op: Clear,
                store_op: DontCare,
            },
            // Normal buffer
            normals: {
                format: Format::R16G16B16A16_SFLOAT,
                samples: 1,
                load_op: Clear,
                store_op: DontCare,
            },
            // Depth buffer
            depth_stencil: {
                format: Format::D32_SFLOAT,
                samples: 1,
                load_op: Clear,
                store_op: DontCare,
            },
        },
        passes: [
            // Write to the diffuse, normals and depth attachments.
            {
                color: [diffuse, normals],
                depth_stencil: {depth_stencil},
                input: [],
            },
            // Apply lighting by reading these three attachments and writing to `final_color`.
            {
                color: [final_color],
                depth_stencil: {},
                input: [diffuse, normals, depth_stencil],
            },
        ],
    )
    .unwrap()
}

pub fn create_deferred_framebuffers_from_images(
    images: &[Arc<Image>],
    render_pass: Arc<RenderPass>,
    allocator: &Allocators,
) -> (FramebufferAttachments, Vec<Arc<Framebuffer>>) {
    let extent = images[0].extent();
    let diffuse_attachment = ImageView::new_default(
        Image::new(
            allocator.memory.clone(),
            ImageCreateInfo {
                extent,
                format: Format::A2B10G10R10_UNORM_PACK32,
                usage: ImageUsage::COLOR_ATTACHMENT
                    | ImageUsage::TRANSIENT_ATTACHMENT
                    | ImageUsage::INPUT_ATTACHMENT,
                ..Default::default()
            },
            Default::default(),
        )
        .unwrap(),
    )
    .unwrap();
    let normals_attachment = ImageView::new_default(
        Image::new(
            allocator.memory.clone(),
            ImageCreateInfo {
                extent,
                format: Format::R16G16B16A16_SFLOAT,
                usage: ImageUsage::COLOR_ATTACHMENT
                    | ImageUsage::TRANSIENT_ATTACHMENT
                    | ImageUsage::INPUT_ATTACHMENT,
                ..Default::default()
            },
            Default::default(),
        )
        .unwrap(),
    )
    .unwrap();
    let depth_attachment = ImageView::new_default(
        Image::new(
            allocator.memory.clone(),
            ImageCreateInfo {
                extent,
                format: Format::D32_SFLOAT,
                usage: ImageUsage::DEPTH_STENCIL_ATTACHMENT
                    | ImageUsage::TRANSIENT_ATTACHMENT
                    | ImageUsage::INPUT_ATTACHMENT,
                ..Default::default()
            },
            Default::default(),
        )
        .unwrap(),
    )
    .unwrap();

    let framebuffers = images
        .iter()
        .map(|image| {
            let view = ImageView::new_default(image.clone()).unwrap();
            Framebuffer::new(
                render_pass.clone(),
                FramebufferCreateInfo {
                    attachments: vec![
                        view,
                        diffuse_attachment.clone(),
                        normals_attachment.clone(),
                        depth_attachment.clone(),
                    ],
                    ..Default::default()
                },
            )
            .unwrap()
        })
        .collect();

    (
        (diffuse_attachment, normals_attachment, depth_attachment),
        framebuffers,
    )
}

pub type FramebufferAttachments = (Arc<ImageView>, Arc<ImageView>, Arc<ImageView>);
