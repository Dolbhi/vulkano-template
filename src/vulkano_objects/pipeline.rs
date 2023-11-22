use std::sync::Arc;

use vulkano::{
    descriptor_set::layout::DescriptorType,
    device::Device,
    pipeline::graphics::{
        color_blend::ColorBlendState,
        depth_stencil::{DepthState, DepthStencilState},
        vertex_input::{Vertex, VertexDefinition},
        viewport::{Viewport, ViewportState},
        GraphicsPipelineCreateInfo,
    },
    pipeline::{
        graphics::rasterization::{CullMode, RasterizationState},
        layout::PipelineDescriptorSetLayoutCreateInfo,
        GraphicsPipeline, Pipeline, PipelineLayout, PipelineShaderStageCreateInfo,
    },
    render_pass::{RenderPass, Subpass},
    shader::EntryPoint,
};

use crate::VertexFull;

/// Pipeline wrapper to handle its own recreation
pub struct PipelineWrapper {
    vs: EntryPoint,
    fs: EntryPoint,
    pub pipeline: Arc<GraphicsPipeline>,
}

impl PipelineWrapper {
    pub fn new(
        device: Arc<Device>,
        vs: EntryPoint,
        fs: EntryPoint,
        viewport: Viewport,
        render_pass: Arc<RenderPass>,
    ) -> Self {
        let pipeline =
            window_size_dependent_pipeline(device, vs.clone(), fs.clone(), viewport, render_pass);
        Self { vs, fs, pipeline }
    }

    pub fn layout(&self) -> &Arc<PipelineLayout> {
        self.pipeline.layout()
    }

    pub fn recreate_pipeline(
        &mut self,
        device: Arc<Device>,
        render_pass: Arc<RenderPass>,
        viewport: Viewport,
    ) {
        self.pipeline = window_size_dependent_pipeline(
            device,
            self.vs.clone(),
            self.fs.clone(),
            viewport,
            render_pass,
        );
    }
}

/// Create pipeline made for rarely size changing windows, with the 2nd binding on the 1st set being dynamic
///
/// ### Descriptor Set Layout
/// Descriptor set 0, binding 0 and 1 are set to dynamic
///
/// ### Pipeline Sates
/// - vertex input: `VertexFull`
/// - viewport: given
/// - rasterization: culls back faces
/// - depth stencil: simple
fn window_size_dependent_pipeline(
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
    // set set 0, binding 1 and 2 to dynamic
    let layout = {
        let mut layout_create_info = PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages);
        layout_create_info.set_layouts[0]
            .bindings
            .get_mut(&0)
            .unwrap()
            .descriptor_type = DescriptorType::UniformBufferDynamic;
        layout_create_info.set_layouts[0]
            .bindings
            .get_mut(&1)
            .unwrap()
            .descriptor_type = DescriptorType::UniformBufferDynamic;

        PipelineLayout::new(
            device.clone(),
            layout_create_info
                .into_pipeline_layout_create_info(device.clone())
                .unwrap(),
        )
        .unwrap()
    };
    let subpass = Subpass::from(render_pass, 0).unwrap();

    GraphicsPipeline::new(
        device,
        None,
        GraphicsPipelineCreateInfo {
            stages: stages.into_iter().collect(),
            vertex_input_state: Some(vertex_input_state),
            input_assembly_state: Some(Default::default()),
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
            rasterization_state: Some(RasterizationState {
                cull_mode: CullMode::Back,
                ..Default::default()
            }),
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
