use std::sync::Arc;

use vulkano::{
    buffer::{BufferUsage, Subbuffer},
    command_buffer::AutoCommandBufferBuilder,
    descriptor_set::{DescriptorSetWithOffsets, PersistentDescriptorSet, WriteDescriptorSet},
    pipeline::{
        graphics::vertex_input::{Vertex, VertexDefinition},
        PipelineBindPoint, PipelineShaderStageCreateInfo,
    },
    render_pass::Subpass,
    sync::GpuFuture,
};

use crate::{
    render::Context,
    shaders,
    vulkano_objects::{
        buffers::create_device_local_buffer,
        pipeline::{
            mod_to_stages, window_size_dependent_pipeline_info, LayoutOverrides, PipelineHandler,
        },
        render_pass::FramebufferAttachments,
    },
    Vertex2d,
};

pub struct LightingSystem {
    pub point_pipeline: PipelineHandler,
    pub direction_pipeline: PipelineHandler,
    ambient_pipeline: PipelineHandler,
    // frame_data: Vec<FrameData>,
    screen_vertices: Subbuffer<[Vertex2d]>,
    point_vertices: Subbuffer<[Vertex2d]>,
    attachments_set: Arc<PersistentDescriptorSet>,
    ambient_color: [f32; 4],
}

impl LightingSystem {
    fn create_lighting_pipeline(
        context: &Context,
        subpass: Subpass,
        stages: [PipelineShaderStageCreateInfo; 2],
        layout_override: &LayoutOverrides,
    ) -> PipelineHandler {
        let vertex_input_state = Vertex2d::per_vertex()
            .definition(&stages[0].entry_point.info().input_interface)
            .unwrap();

        let layout = layout_override.create_layout(context.device.clone(), &stages);

        PipelineHandler::new(
            context.device.clone(),
            window_size_dependent_pipeline_info(
                stages,
                layout,
                vertex_input_state,
                context.viewport.clone(),
                subpass,
                crate::vulkano_objects::pipeline::PipelineType::Lighting,
            ),
        )
    }
    /// Returned layouts are in order: [global, point, directional]
    pub fn new(
        context: &Context,
        subpass: &Subpass,
        attachments: &FramebufferAttachments,
        layout_override: &LayoutOverrides,
    ) -> Self {
        // create pipelines
        let point_pipeline = Self::create_lighting_pipeline(
            context,
            subpass.clone(),
            mod_to_stages(
                context.device.clone(),
                shaders::load_point_vs,
                shaders::load_point_fs,
            ),
            layout_override,
        );

        let direction_pipeline = Self::create_lighting_pipeline(
            context,
            subpass.clone(),
            mod_to_stages(
                context.device.clone(),
                shaders::load_direction_vs,
                shaders::load_direction_fs,
            ),
            layout_override,
        );

        let ambient_pipeline = Self::create_lighting_pipeline(
            context,
            subpass.clone(),
            mod_to_stages(
                context.device.clone(),
                shaders::load_direction_vs,
                shaders::load_ambient_fs,
            ),
            layout_override,
        );

        // let image_count = context.get_image_count();

        // create buffers and descriptor sets
        let attachments_set = Self::create_attachment_set(&point_pipeline, context, attachments);

        let (screen_vertices, vertex_future) = create_device_local_buffer(
            &context.allocators,
            context.queue.clone(),
            vec![
                Vertex2d {
                    position: [-1.0, -1.0],
                },
                Vertex2d {
                    position: [-1.0, 3.0],
                },
                Vertex2d {
                    position: [3.0, -1.0],
                },
            ],
            BufferUsage::VERTEX_BUFFER,
        );
        let (point_vertices, point_future) = create_device_local_buffer(
            &context.allocators,
            context.queue.clone(),
            vec![
                Vertex2d {
                    position: [-1.0, -1.0],
                },
                Vertex2d {
                    position: [-1.0, 1.0],
                },
                Vertex2d {
                    position: [1.0, -1.0],
                },
                Vertex2d {
                    position: [1.0, -1.0],
                },
                Vertex2d {
                    position: [-1.0, 1.0],
                },
                Vertex2d {
                    position: [1.0, 1.0],
                },
            ],
            BufferUsage::VERTEX_BUFFER,
        );

        let fence = vertex_future
            .join(point_future)
            .then_signal_fence_and_flush()
            .unwrap();
        fence.wait(None).unwrap();

        // // global, point, dir
        // let layouts = [
        //     point_pipeline
        //         .layout()
        //         .set_layouts()
        //         .get(0)
        //         .unwrap()
        //         .clone(),
        //     point_pipeline
        //         .layout()
        //         .set_layouts()
        //         .get(2)
        //         .unwrap()
        //         .clone(),
        //     direction_pipeline
        //         .layout()
        //         .set_layouts()
        //         .get(2)
        //         .unwrap()
        //         .clone(),
        // ];

        LightingSystem {
            point_pipeline,
            direction_pipeline,
            ambient_pipeline,
            // frame_data,
            screen_vertices,
            point_vertices,
            attachments_set,
            ambient_color: [0., 0., 0., 0.],
        }
    }

    pub fn recreate_pipeline(&mut self, context: &Context) {
        self.point_pipeline
            .recreate_pipeline(context.device.clone(), context.viewport.clone());
        self.direction_pipeline
            .recreate_pipeline(context.device.clone(), context.viewport.clone());
        self.ambient_pipeline
            .recreate_pipeline(context.device.clone(), context.viewport.clone());
    }
    /// recreate the descriptor set describing the framebuffer attachments, must be done after recreating framebuffer (see `DrawSystem::recreate_pipelines`)
    pub fn recreate_descriptor(&mut self, context: &Context, attachments: &FramebufferAttachments) {
        self.attachments_set =
            Self::create_attachment_set(&self.point_pipeline, context, attachments);
    }
    fn create_attachment_set(
        pipeline: &PipelineHandler,
        context: &Context,
        attachments: &FramebufferAttachments,
    ) -> Arc<PersistentDescriptorSet> {
        PersistentDescriptorSet::new(
            &context.allocators.descriptor_set,
            pipeline.layout().set_layouts().get(1).unwrap().clone(),
            [
                WriteDescriptorSet::image_view(0, attachments.0.clone()),
                WriteDescriptorSet::image_view(1, attachments.1.clone()),
                WriteDescriptorSet::image_view(2, attachments.2.clone()),
            ],
            [],
        )
        .unwrap()
    }

    pub fn set_ambient_color(&mut self, ambient_color: impl Into<[f32; 4]>) {
        // self.frame_data[image_i].update(
        //     point_lights.into_iter(),
        //     dir_lights.into_iter(),
        //     global_data,
        // );
        self.ambient_color = ambient_color.into();
    }

    pub fn render<P, A: vulkano::command_buffer::allocator::CommandBufferAllocator>(
        &mut self,
        // image_i: usize,
        global_set: DescriptorSetWithOffsets,
        point_set: DescriptorSetWithOffsets,
        dir_set: DescriptorSetWithOffsets,
        last_point_index: Option<usize>,
        last_dir_index: Option<usize>,
        command_builder: &mut AutoCommandBufferBuilder<P, A>,
    ) {
        // NOTE: descriptor set order: global, attachments, point/direction

        // println!(
        //     "Pipeline layout: {:?}",
        //     self.pipeline.layout().set_layouts()[1]
        // );
        // println!(
        //     "Set layout:      {:?}",
        //     frame.descriptor_sets[0].as_ref().0.layout()
        // );

        // let global_attachments = vec![global_set.clone(), self.attachments_set.clone().into()];

        // bind commands
        // point lights
        if let Some(last_index) = last_point_index {
            let pipeline = &self.point_pipeline.pipeline;
            let layout = self.point_pipeline.layout();
            command_builder
                .bind_pipeline_graphics(pipeline.clone())
                .unwrap()
                .bind_descriptor_sets(
                    PipelineBindPoint::Graphics,
                    layout.clone(),
                    0,
                    vec![
                        global_set.clone(),
                        self.attachments_set.clone().into(),
                        point_set,
                    ],
                )
                .unwrap()
                .bind_vertex_buffers(0, self.point_vertices.clone())
                .unwrap()
                .draw(
                    self.point_vertices.len() as u32,
                    last_index as u32 + 1,
                    0,
                    0,
                )
                .unwrap();
        }
        // directional lights
        if let Some(last_index) = last_dir_index {
            let pipeline = &self.direction_pipeline.pipeline;
            let layout = self.direction_pipeline.layout();
            command_builder
                .bind_pipeline_graphics(pipeline.clone())
                .unwrap()
                .bind_descriptor_sets(
                    PipelineBindPoint::Graphics,
                    layout.clone(),
                    0,
                    vec![
                        global_set.clone(),
                        self.attachments_set.clone().into(),
                        dir_set,
                    ],
                )
                .unwrap()
                .bind_vertex_buffers(0, self.screen_vertices.clone())
                .unwrap()
                .draw(
                    self.screen_vertices.len() as u32,
                    last_index as u32 + 1,
                    0,
                    0,
                )
                .unwrap();
        }
        // ambient light
        let pipeline = &self.ambient_pipeline.pipeline;
        let layout = self.ambient_pipeline.layout();
        command_builder
            .bind_pipeline_graphics(pipeline.clone())
            .unwrap()
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                layout.clone(),
                0,
                vec![global_set.clone(), self.attachments_set.clone().into()],
            )
            .unwrap()
            .push_constants(
                layout.clone(),
                0,
                shaders::GPUAmbientData {
                    ambient_color: self.ambient_color,
                },
            )
            .unwrap()
            .bind_vertex_buffers(0, self.screen_vertices.clone())
            .unwrap()
            .draw(self.screen_vertices.len() as u32, 1, 0, 0)
            .unwrap();
    }
}
