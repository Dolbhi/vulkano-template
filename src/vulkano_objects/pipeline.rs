//! Pipeline handling and creating
//! NOT reusable for multiple renderers (mostly)

use std::{fmt::Debug, sync::Arc};

use vulkano::{
    buffer::{BufferContents, Subbuffer},
    descriptor_set::{
        allocator::DescriptorSetAllocator,
        layout::{DescriptorSetLayoutBinding, DescriptorSetLayoutCreateInfo},
        PersistentDescriptorSet, WriteDescriptorSet,
    },
    device::Device,
    pipeline::{
        graphics::{
            color_blend::{
                AttachmentBlend, BlendFactor, BlendOp, ColorBlendAttachmentState, ColorBlendState,
            },
            depth_stencil::{DepthState, DepthStencilState},
            rasterization::{CullMode, RasterizationState},
            vertex_input::VertexInputState,
            viewport::{Viewport, ViewportState},
            GraphicsPipelineCreateInfo,
        },
        layout::PipelineDescriptorSetLayoutCreateInfo,
        GraphicsPipeline, Pipeline, PipelineLayout, PipelineShaderStageCreateInfo,
    },
    render_pass::Subpass,
    shader::{ShaderModule, ShaderStages},
};

use super::{allocators::Allocators, buffers::create_storage_buffer};

/// Pipeline wrapper to handle its own recreation
pub struct PipelineHandler {
    pub create_info: GraphicsPipelineCreateInfo,
    pub pipeline: Arc<GraphicsPipeline>,
}

#[derive(Clone, Copy)]
pub enum PipelineType {
    Drawing,
    Lighting,
}

#[derive(Clone, Default)]
/// Set layouts to be replaced in a `PipelineDescriptorSetLayoutCreateInfo`
pub struct LayoutOverrides {
    pub set_layout_overrides: Vec<(usize, DescriptorSetLayoutCreateInfo)>,
}

impl PipelineHandler {
    pub fn new(device: Arc<Device>, create_info: GraphicsPipelineCreateInfo) -> Self {
        let pipeline = GraphicsPipeline::new(device, None, create_info.clone()).unwrap();

        Self {
            create_info,
            pipeline,
        }
    }

    pub fn create_storage_buffer<T: BufferContents>(
        &self,
        allocators: &Allocators,
        object_count: usize,
        set: usize,
    ) -> (Subbuffer<[T]>, Arc<PersistentDescriptorSet>) {
        let layout = self.layout().set_layouts()[set].clone();

        create_storage_buffer(allocators, layout, object_count)
    }

    /// Creates descriptor set with single buffer on binding 0
    pub fn create_descriptor_set<A, T: BufferContents>(
        &self,
        allocator: &A,
        buffer: Subbuffer<T>,
        set: usize,
    ) -> Arc<PersistentDescriptorSet<A::Alloc>>
    where
        A: DescriptorSetAllocator + ?Sized,
    {
        PersistentDescriptorSet::new(
            allocator,
            self.layout().set_layouts()[set].clone(),
            [WriteDescriptorSet::buffer(0, buffer)],
            [],
        )
        .unwrap()
    }

    pub fn layout(&self) -> &Arc<PipelineLayout> {
        self.pipeline.layout()
    }

    /// recreate pipeline with cached shader entry points, subpass, dynamic bindings and pipeline type with new viewport
    pub fn recreate_pipeline(&mut self, device: Arc<Device>, viewport: Viewport) {
        if let Some(mut view_state) = self.create_info.viewport_state.take() {
            view_state.viewports[0].extent = viewport.extent;
            self.create_info.viewport_state = Some(view_state);
        }

        self.pipeline = GraphicsPipeline::new(device, None, self.create_info.clone()).unwrap();
    }
}

impl PipelineType {
    /// assigns the following members for the given pipeline create info
    /// - rasterization_state
    /// - depth_stencil_state
    /// - multisample_state
    /// - color_blend_state
    /// - subpass (Not anymore)
    fn apply(
        self,
        create_info: GraphicsPipelineCreateInfo,
        subpass: Subpass,
    ) -> GraphicsPipelineCreateInfo {
        match self {
            Self::Drawing => {
                // let subpass = Subpass::from(render_pass, 0).unwrap();
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
                // let subpass = Subpass::from(render_pass, 1).unwrap();
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

impl LayoutOverrides {
    pub fn add_set(&mut self, index: usize, set_layout_info: DescriptorSetLayoutCreateInfo) {
        self.set_layout_overrides.push((index, set_layout_info));
    }

    /// create pipeline layout using given stages with overrides
    pub fn create_layout(
        &self,
        device: Arc<Device>,
        stages: &[PipelineShaderStageCreateInfo; 2],
    ) -> Arc<PipelineLayout> {
        let draw_layout_info =
            self.apply(PipelineDescriptorSetLayoutCreateInfo::from_stages(stages));

        PipelineLayout::new(
            device.clone(),
            draw_layout_info
                .into_pipeline_layout_create_info(device)
                .unwrap(),
        )
        .unwrap()
    }

    pub fn apply(
        &self,
        mut create_info: PipelineDescriptorSetLayoutCreateInfo,
    ) -> PipelineDescriptorSetLayoutCreateInfo {
        for (set, layout) in self.set_layout_overrides.iter() {
            create_info.set_layouts[*set] = layout.clone();
        }
        create_info
    }

    /// Creates a `DescriptorSetLayoutCreateInfo` with a single uniform buffer at binding 0
    pub fn single_uniform_set(stages: ShaderStages) -> DescriptorSetLayoutCreateInfo {
        let mut binding = DescriptorSetLayoutBinding::descriptor_type(
            vulkano::descriptor_set::layout::DescriptorType::UniformBuffer,
        );
        binding.stages = stages;

        DescriptorSetLayoutCreateInfo {
            bindings: [(0, binding)].into(),
            ..Default::default()
        }
    }
}

pub fn mod_to_stages<T: Debug>(
    device: Arc<Device>,
    vs: impl FnOnce(Arc<Device>) -> Result<Arc<ShaderModule>, T>,
    fs: impl FnOnce(Arc<Device>) -> Result<Arc<ShaderModule>, T>, // stages: [dyn FnOnce(Arc<Device>) -> Result<Arc<ShaderModule>, T>; 2],
) -> [PipelineShaderStageCreateInfo; 2] {
    [vs(device.clone()), fs(device.clone())].map(|module| {
        PipelineShaderStageCreateInfo::new(module.unwrap().entry_point("main").unwrap())
    })
}

/// Create pipeline made for rarely size changing windows
///
/// ### Pipeline States
/// - vertex input: based on given generic
/// - viewport: given
pub fn window_size_dependent_pipeline_info(
    stages: impl IntoIterator<Item = PipelineShaderStageCreateInfo>,
    layout: Arc<PipelineLayout>,
    vertex_input_state: VertexInputState,
    viewport: Viewport,
    subpass: Subpass,
    pipeline_type: PipelineType,
) -> GraphicsPipelineCreateInfo {
    pipeline_type.apply(
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
        subpass,
    )
}
