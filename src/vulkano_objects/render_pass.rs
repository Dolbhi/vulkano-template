use std::sync::Arc;

use vulkano::device::Device;
use vulkano::format::Format::D16_UNORM;
use vulkano::render_pass::RenderPass;
use vulkano::swapchain::Swapchain;

pub fn create_render_pass(device: Arc<Device>, swapchain: Arc<Swapchain>) -> Arc<RenderPass> {
    vulkano::single_pass_renderpass!(
        device,
        attachments: {
            color: {
                load: Clear,
                store: Store,
                format: swapchain.image_format(),
                samples: 1,
            },
            depth_stencil: {
                load: Clear,
                store: DontCare,
                format: D16_UNORM,
                samples: 1,
            }
        },
        pass: {
            color: [color],
            depth_stencil: {depth_stencil},
        },
    )
    .unwrap()
}
