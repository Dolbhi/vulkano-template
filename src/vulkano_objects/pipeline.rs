use std::{marker::PhantomData, sync::Arc};

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

/// Pipeline wrapper to handle its own recreation
pub struct PipelineHandler<V: Vertex> {
    vs: EntryPoint,
    fs: EntryPoint,
    pub pipeline: Arc<GraphicsPipeline>,
    vertex_type: PhantomData<V>,
    dynamic_bindings: Vec<(usize, u32)>,
}

impl<V: Vertex> PipelineHandler<V> {
    pub fn new(
        device: Arc<Device>,
        vs: EntryPoint,
        fs: EntryPoint,
        viewport: Viewport,
        render_pass: Arc<RenderPass>,
        dynamic_bindings: impl IntoIterator<Item = (usize, u32)> + Clone,
    ) -> Self {
        let pipeline = window_size_dependent_pipeline::<V>(
            device,
            vs.clone(),
            fs.clone(),
            viewport,
            render_pass,
            dynamic_bindings.clone(),
        );
        Self {
            vs,
            fs,
            pipeline,
            vertex_type: PhantomData,
            dynamic_bindings: dynamic_bindings.into_iter().collect(),
        }
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
        self.pipeline = window_size_dependent_pipeline::<V>(
            device,
            self.vs.clone(),
            self.fs.clone(),
            viewport,
            render_pass,
            self.dynamic_bindings.clone(),
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
pub fn window_size_dependent_pipeline<V: Vertex>(
    device: Arc<Device>,
    vs: EntryPoint,
    fs: EntryPoint,
    viewport: Viewport,
    // images: &[Arc<Image>],
    render_pass: Arc<RenderPass>,
    dynamic_bindings: impl IntoIterator<Item = (usize, u32)>,
) -> Arc<GraphicsPipeline> {
    let vertex_input_state = V::per_vertex()
        .definition(&vs.info().input_interface) //[Position::per_vertex(), Normal::per_vertex()]
        .unwrap();
    let stages = [
        PipelineShaderStageCreateInfo::new(vs),
        PipelineShaderStageCreateInfo::new(fs),
    ];
    let layout = {
        let mut layout_create_info = PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages);
        for (set, binding) in dynamic_bindings {
            layout_create_info.set_layouts[set]
                .bindings
                .get_mut(&binding)
                .unwrap()
                .descriptor_type = DescriptorType::UniformBufferDynamic;
            // println!("Making set {}, binding {} dynamic", set, binding);
        }

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
                    extent: viewport.extent,
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
