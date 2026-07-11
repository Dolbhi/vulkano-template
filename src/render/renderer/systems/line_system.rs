// use cgmath::{Matrix4, Vector3, Vector4};
use vulkano::{
    buffer::{BufferUsage, Subbuffer},
    command_buffer::AutoCommandBufferBuilder,
    descriptor_set::DescriptorSetWithOffsets,
    pipeline::{
        graphics::vertex_input::{Vertex, VertexDefinition},
        PipelineBindPoint,
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
            PipelineType,
        },
    },
    Vertex3d,
};

pub struct LineSystem {
    pub pipeline: PipelineHandler,
    box_mesh: Subbuffer<[Vertex3d]>,
    line_mesh: Subbuffer<[Vertex3d]>,
}

impl LineSystem {
    pub fn new(context: &Context, subpass: &Subpass, layout_overrides: &LayoutOverrides) -> Self {
        let stages = mod_to_stages(
            context.device.clone(),
            shaders::load_bounding_box_vs,
            shaders::load_bounding_box_fs,
        );

        let vertex_input_state = Vertex3d::per_vertex()
            .definition(&stages[0].entry_point.info().input_interface) //[Position::per_vertex(), Normal::per_vertex()]
            .unwrap();
        let layout = layout_overrides.create_layout(context.device.clone(), &stages);

        // bounding box mesh
        let (box_line_list, box_future) = create_device_local_buffer(
            &context.allocators,
            context.queue.clone(),
            [
                [0., 0., 0.],
                [1., 0., 0.],
                [0., 0., 0.],
                [0., 1., 0.],
                [0., 0., 0.],
                [0., 0., 1.],
                [1., 0., 0.],
                [1., 1., 0.],
                [1., 0., 0.],
                [1., 0., 1.],
                [0., 1., 0.],
                [1., 1., 0.],
                [0., 1., 0.],
                [0., 1., 1.],
                [0., 0., 1.],
                [1., 0., 1.],
                [0., 0., 1.],
                [0., 1., 1.],
                [1., 1., 0.],
                [1., 1., 1.],
                [1., 0., 1.],
                [1., 1., 1.],
                [0., 1., 1.],
                [1., 1., 1.],
            ]
            .map(|p| p.into()),
            BufferUsage::VERTEX_BUFFER,
        );
        // single line mesh
        let (single_line_list, line_future) = create_device_local_buffer(
            &context.allocators,
            context.queue.clone(),
            [[0., 0., 0.], [1., 1., 1.]].map(|p| p.into()),
            BufferUsage::VERTEX_BUFFER,
        );
        // send it
        box_future
            .join(line_future)
            .then_signal_fence_and_flush()
            .unwrap()
            .wait(None)
            .unwrap();

        LineSystem {
            pipeline: PipelineHandler::new(
                context.device.clone(),
                window_size_dependent_pipeline_info(
                    stages,
                    layout,
                    vertex_input_state,
                    context.viewport.clone(),
                    subpass.clone(),
                    PipelineType::Lines,
                ),
            ),
            box_mesh: box_line_list,
            line_mesh: single_line_list,
        }
    }

    /// Recreate all pipelines with any changes in viewport
    ///
    /// See also: [recreate_pipeline](PipelineHandler::recreate_pipeline)
    pub fn recreate_pipelines(&mut self, context: &Context) {
        self.pipeline
            .recreate_pipeline(context.device.clone(), context.viewport.clone())
    }

    // pub fn bounding_box_to_transform(
    //     min: Vector3<f32>,
    //     max: Vector3<f32>,
    //     colour: Vector4<f32>,
    // ) -> (Matrix4<f32>, Vector4<f32>) {
    //     let (x, y, z) = (max - min).into();
    //     let transform = Matrix4::from_translation(min) * Matrix4::from_nonuniform_scale(x, y, z);
    //     (transform, colour)
    // }

    pub fn render<P, A: vulkano::command_buffer::allocator::CommandBufferAllocator>(
        &mut self,
        // image_i: usize,
        global_set: DescriptorSetWithOffsets,
        box_set: DescriptorSetWithOffsets,
        last_box_index: Option<usize>,
        last_line_index: Option<usize>,
        command_builder: &mut AutoCommandBufferBuilder<P, A>,
    ) {
        // bind commands
        let mut first_line_index = 0;
        if let Some(last_index) = last_box_index {
            first_line_index = last_index + 1;
            let pipeline = &self.pipeline.pipeline;
            let layout = self.pipeline.layout();
            command_builder
                .bind_pipeline_graphics(pipeline.clone())
                .unwrap()
                .bind_descriptor_sets(
                    PipelineBindPoint::Graphics,
                    layout.clone(),
                    0,
                    vec![global_set.clone(), box_set.clone()],
                )
                .unwrap()
                .bind_vertex_buffers(0, self.box_mesh.clone())
                .unwrap()
                .draw(self.box_mesh.len() as u32, last_index as u32 + 1, 0, 0)
                .unwrap();
        }
        if let Some(last_index) = last_line_index {
            let pipeline = &self.pipeline.pipeline;
            let layout = self.pipeline.layout();
            command_builder
                .bind_pipeline_graphics(pipeline.clone())
                .unwrap()
                .bind_descriptor_sets(
                    PipelineBindPoint::Graphics,
                    layout.clone(),
                    0,
                    vec![global_set, box_set],
                )
                .unwrap()
                .bind_vertex_buffers(0, self.line_mesh.clone())
                .unwrap()
                .draw(
                    self.line_mesh.len() as u32,
                    (last_index - first_line_index) as u32 + 1,
                    0,
                    first_line_index as u32,
                )
                .unwrap();
        }
    }
}
