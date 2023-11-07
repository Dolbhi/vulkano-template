use std::sync::Arc;

use vulkano::device::Device;
use vulkano::format::Format;
use vulkano::render_pass::RenderPass;
use vulkano::swapchain::Swapchain;

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
