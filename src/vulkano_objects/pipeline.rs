//! Pipeline handling and creating
//! NOT reusable for multiple renderers (mostly)

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
        graphics::{
            color_blend::{AttachmentBlend, BlendFactor, BlendOp, ColorBlendAttachmentState},
            rasterization::{CullMode, RasterizationState},
        },
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
    pipeline_type: PipelineType,
}

impl<V: Vertex> PipelineHandler<V> {
    pub fn new(
        device: Arc<Device>,
        vs: EntryPoint,
        fs: EntryPoint,
        viewport: Viewport,
        render_pass: Arc<RenderPass>,
        dynamic_bindings: impl IntoIterator<Item = (usize, u32)> + Clone,
        pipeline_type: PipelineType,
    ) -> Self {
        let pipeline = window_size_dependent_pipeline::<V>(
            device,
            vs.clone(),
            fs.clone(),
            viewport,
            render_pass,
            dynamic_bindings.clone(),
            pipeline_type,
        );
        Self {
            vs,
            fs,
            pipeline,
            vertex_type: PhantomData,
            dynamic_bindings: dynamic_bindings.into_iter().collect(),
            pipeline_type,
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
            self.pipeline_type,
        );
    }
}

#[derive(Clone, Copy)]
pub enum PipelineType {
    Drawing,
    Lighting,
}

impl PipelineType {
    /// assigns the following members for the given pipeline create info
    /// - rasterization_state
    /// - depth_stencil_state
    /// - multisample_state
    /// - color_blend_state
    /// - subpass
    fn apply_to_create_info(
        self,
        create_info: GraphicsPipelineCreateInfo,
        render_pass: Arc<RenderPass>,
    ) -> GraphicsPipelineCreateInfo {
        match self {
            Self::Drawing => {
                let subpass = Subpass::from(render_pass, 0).unwrap();
                GraphicsPipelineCreateInfo {
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
                    ..create_info
                }
            }
            Self::Lighting => {
                let subpass = Subpass::from(render_pass, 1).unwrap();
                GraphicsPipelineCreateInfo {
                    rasterization_state: Some(Default::default()),
                    depth_stencil_state: None,
                    multisample_state: Some(Default::default()),
                    color_blend_state: Some(ColorBlendState::with_attachment_states(
                        // additive blending (not needed as all lighting is done in one draw call)
                        subpass.num_color_attachments(),
                        ColorBlendAttachmentState {
                            blend: Some(AttachmentBlend {
                                color_blend_op: BlendOp::Add,
                                src_color_blend_factor: BlendFactor::One,
                                dst_color_blend_factor: BlendFactor::One,
                                alpha_blend_op: BlendOp::Max,
                                src_alpha_blend_factor: BlendFactor::One,
                                dst_alpha_blend_factor: BlendFactor::One,
                            }),
                            ..Default::default()
                        },
                    )),
                    subpass: Some(subpass.into()),
                    ..create_info
                }
            }
        }
    }
}

/// Create pipeline made for rarely size changing windows
///
/// ### Pipeline Sates
/// - vertex input: based on given generic
/// - viewport: given
fn window_size_dependent_pipeline<V: Vertex>(
    device: Arc<Device>,
    vs: EntryPoint,
    fs: EntryPoint,
    viewport: Viewport,
    // images: &[Arc<Image>],
    render_pass: Arc<RenderPass>,
    dynamic_bindings: impl IntoIterator<Item = (usize, u32)>,
    pipeline_type: PipelineType,
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

    GraphicsPipeline::new(
        device,
        None,
        pipeline_type.apply_to_create_info(
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
                ..GraphicsPipelineCreateInfo::layout(layout)
            },
            render_pass,
        ),
    )
    .unwrap()
}
