use std::sync::Arc;

use vulkano::device::Device;
use vulkano::pipeline::graphics::color_blend::ColorBlendState;
use vulkano::pipeline::graphics::depth_stencil::{DepthState, DepthStencilState};
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::rasterization::RasterizationState;
use vulkano::pipeline::graphics::vertex_input::{Vertex, VertexDefinition};
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::pipeline::graphics::GraphicsPipelineCreateInfo;
use vulkano::pipeline::layout::PipelineDescriptorSetLayoutCreateInfo;
use vulkano::pipeline::{GraphicsPipeline, PipelineLayout, PipelineShaderStageCreateInfo};
use vulkano::render_pass::{RenderPass, Subpass};
use vulkano::shader::EntryPoint;

use crate::VertexFull;

// pub fn create_pipeline(
//     device: Arc<Device>,
//     vs: Arc<ShaderModule>,
//     fs: Arc<ShaderModule>,
//     render_pass: Arc<RenderPass>,
//     viewport: Viewport,
//     // assembly topology
//     // rasterization polygon mode
// ) -> Arc<GraphicsPipeline> {
//     GraphicsPipeline::new(
//         device,
//         None,
//         GraphicsPipelineCreateInfo {
//             vertex_input_state: VertexFull::per_vertex().,
//             depth_stencil_state: Some(DepthStencilState::simple_depth_test()),
//             ..Default::default()
//         },
//     )
//     .unwrap()

//     // .vertex_shader(vs.entry_point("main").unwrap(), ())
//     // .depth_stencil_state(DepthState::simple())
//     // .viewport_state(ViewportState:: {
//     //     viewports: [viewport],
//     //     scissors: [Default::default()],
//     // })
//     // .fragment_shader(fs.entry_point("main").unwrap(), ())
//     // .render_pass(Subpass::from(render_pass, 0).unwrap())
//     // .build(device)
//     // .unwrap()
// }

pub fn window_size_dependent_pipeline(
    device: Arc<Device>,
    vs: EntryPoint,
    fs: EntryPoint,
    viewport: Viewport,
    // images: &[Arc<Image>],
    render_pass: Arc<RenderPass>,
) -> Arc<GraphicsPipeline> {
    // let device = memory_allocator.device().clone();
    // let extent = images[0].extent();

    // let depth_buffer = ImageView::new_default(
    //     Image::new(
    //         memory_allocator,
    //         ImageCreateInfo {
    //             image_type: ImageType::Dim2d,
    //             format: Format::D16_UNORM,
    //             extent: images[0].extent(),
    //             usage: ImageUsage::DEPTH_STENCIL_ATTACHMENT | ImageUsage::TRANSIENT_ATTACHMENT,
    //             ..Default::default()
    //         },
    //         AllocationCreateInfo::default(),
    //     )
    //     .unwrap(),
    // )
    // .unwrap();

    // let framebuffers = images
    //     .iter()
    //     .map(|image| {
    //         let view = ImageView::new_default(image.clone()).unwrap();
    //         Framebuffer::new(
    //             render_pass.clone(),
    //             FramebufferCreateInfo {
    //                 attachments: vec![view, depth_buffer.clone()],
    //                 ..Default::default()
    //             },
    //         )
    //         .unwrap()
    //     })
    //     .collect::<Vec<_>>();

    // In the triangle example we use a dynamic viewport, as its a simple example. However in the
    // teapot example, we recreate the pipelines with a hardcoded viewport instead. This allows the
    // driver to optimize things, at the cost of slower window resizes.
    // https://computergraphics.stackexchange.com/questions/5742/vulkan-best-way-of-updating-pipeline-viewport
    let vertex_input_state = VertexFull::per_vertex()
        .definition(&vs.info().input_interface) //[Position::per_vertex(), Normal::per_vertex()]
        .unwrap();
    let stages = [
        PipelineShaderStageCreateInfo::new(vs),
        PipelineShaderStageCreateInfo::new(fs),
    ];
    let layout = PipelineLayout::new(
        device.clone(),
        PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
            .into_pipeline_layout_create_info(device.clone())
            .unwrap(),
    )
    .unwrap();
    let subpass = Subpass::from(render_pass, 0).unwrap();

    GraphicsPipeline::new(
        device,
        None,
        GraphicsPipelineCreateInfo {
            stages: stages.into_iter().collect(),
            vertex_input_state: Some(vertex_input_state),
            input_assembly_state: Some(InputAssemblyState::default()),
            viewport_state: Some(ViewportState {
                viewports: [Viewport {
                    offset: [0.0, 0.0],
                    extent: [viewport.extent[0] as f32, viewport.extent[1] as f32],
                    depth_range: 0.0..=1.0,
                }]
                .into_iter()
                .collect(),
                ..Default::default()
            }),
            rasterization_state: Some(RasterizationState::default()),
            depth_stencil_state: Some(DepthStencilState {
                depth: Some(DepthState::simple()),
                ..Default::default()
            }),
            multisample_state: Some(Default::default()),
            color_blend_state: Some(ColorBlendState::with_attachment_states(
                subpass.num_color_attachments(),
                Default::default(),
            )),
            subpass: Some(subpass.into()),
            ..GraphicsPipelineCreateInfo::layout(layout)
        },
    )
    .unwrap()
}
