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

pub struct BoundingBoxSystem {
    pub pipeline: PipelineHandler,
    line_list: Subbuffer<[Vertex3d]>,
}

impl BoundingBoxSystem {
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
        let (line_list, future) = create_device_local_buffer(
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
        // send it
        future
            .then_signal_fence_and_flush()
            .unwrap()
            .wait(None)
            .unwrap();

        BoundingBoxSystem {
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
            line_list,
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
        command_builder: &mut AutoCommandBufferBuilder<P, A>,
    ) {
        // bind commands
        if let Some(last_index) = last_box_index {
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
                .bind_vertex_buffers(0, self.line_list.clone())
                .unwrap()
                .draw(self.line_list.len() as u32, last_index as u32 + 1, 0, 0)
                .unwrap();
        }
    }
}
