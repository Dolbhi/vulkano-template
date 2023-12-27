use std::sync::Arc;

use vulkano::{
    buffer::{BufferUsage, Subbuffer},
    command_buffer::AutoCommandBufferBuilder,
    descriptor_set::{DescriptorSetWithOffsets, PersistentDescriptorSet, WriteDescriptorSet},
    pipeline::PipelineBindPoint,
    shader::ShaderModule,
    sync::GpuFuture,
};

use crate::{
    shaders::lighting::{
        self,
        DirectionLight,
        GPUGlobalData,
        // fs::{DirectionLight, GPULightingData, PointLight},
        PointLight,
    },
    vulkano_objects::{
        buffers::{create_device_local_buffer, create_global_descriptors, create_storage_buffers},
        pipeline::PipelineHandler,
    },
    Vertex2d,
};

use super::Renderer;

pub struct LightingSystem {
    point_pipeline: PipelineHandler<Vertex2d>,
    direction_pipeline: PipelineHandler<Vertex2d>,
    ambient_pipeline: PipelineHandler<Vertex2d>,
    frame_data: Vec<FrameData>,
    screen_vertices: Subbuffer<[Vertex2d]>,
    point_vertices: Subbuffer<[Vertex2d]>,
    attachments_set: Arc<PersistentDescriptorSet>,
    ambient_color: [f32; 4],
}

impl LightingSystem {
    fn create_lighting_pipeline(
        context: &Renderer,
        vs: Arc<ShaderModule>,
        fs: Arc<ShaderModule>,
        dynamic_bindings: impl IntoIterator<Item = (usize, u32)> + Clone,
    ) -> PipelineHandler<Vertex2d> {
        PipelineHandler::new(
            context.device.clone(),
            vs.entry_point("main").unwrap(),
            fs.entry_point("main").unwrap(),
            context.viewport.clone(),
            context.render_pass.clone(),
            dynamic_bindings,
            crate::vulkano_objects::pipeline::PipelineType::Lighting,
        )
    }
    pub fn new(context: &Renderer) -> Self {
        // let pipeline = {
        //     let vs = lighting::vs::load(context.device.clone())
        //         .expect("failed to create lighting shader module")
        //         .entry_point("main")
        //         .unwrap();
        //     let fs = lighting::fs::load(context.device.clone())
        //         .expect("failed to create lighting shader module")
        //         .entry_point("main")
        //         .unwrap();
        //     PipelineHandler::new(
        //         context.device.clone(),
        //         vs,
        //         fs,
        //         context.viewport.clone(),
        //         context.render_pass.clone(),
        //         [(1, 0)],
        //         crate::vulkano_objects::pipeline::PipelineType::Lighting,
        //     )
        // };

        // create pipelines
        let vs = lighting::load_point_vs(context.device.clone())
            .expect("failed to create point shader module");
        let fs = lighting::load_point_fs(context.device.clone())
            .expect("failed to create point shader module");
        let point_pipeline = Self::create_lighting_pipeline(&context, vs, fs, [(1, 0)]); // global data is dynamic

        let vs = lighting::load_direction_vs(context.device.clone())
            .expect("failed to create directional shader module");
        let fs = lighting::load_direction_fs(context.device.clone())
            .expect("failed to create directional shader module");
        let direction_pipeline = Self::create_lighting_pipeline(&context, vs.clone(), fs, []);

        let fs = lighting::load_ambient_fs(context.device.clone())
            .expect("failed to create ambient shader module");
        let ambient_pipeline = Self::create_lighting_pipeline(&context, vs.clone(), fs, []);

        let image_count = context.swapchain.image_count() as usize;

        // create buffers and descriptor sets
        let attachments_set = Self::create_attachment_set(&point_pipeline, context);

        let mut global_data = create_global_descriptors::<GPUGlobalData>(
            &context.allocators,
            &context.device,
            point_pipeline
                .layout()
                .set_layouts()
                .get(1)
                .unwrap()
                .clone(),
            image_count,
        );
        let mut point_data = create_storage_buffers::<PointLight>(
            &context.allocators,
            point_pipeline
                .layout()
                .set_layouts()
                .get(2)
                .unwrap()
                .clone(),
            image_count,
            1000,
        );
        let mut dir_data = create_storage_buffers::<DirectionLight>(
            &context.allocators,
            direction_pipeline
                .layout()
                .set_layouts()
                .get(1)
                .unwrap()
                .clone(),
            image_count,
            1000,
        );

        // pack into frames
        let mut frame_data = vec![];
        for _ in 0..image_count {
            let (global_buffer, global_set) = global_data.pop().unwrap();
            let (point_buffer, point_set) = point_data.pop().unwrap();
            let (dir_buffer, dir_set) = dir_data.pop().unwrap();

            // println!("Creation layout: {:?}", global_set.as_ref().0.layout());

            frame_data.push(FrameData {
                global_buffer,
                point_buffer,
                dir_buffer,
                global_set,
                point_set: point_set.into(),
                dir_set: dir_set.into(),
                last_point_index: 0,
                last_dir_count: 0,
            });
        }

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

        LightingSystem {
            point_pipeline,
            direction_pipeline,
            ambient_pipeline,
            frame_data,
            screen_vertices,
            point_vertices,
            attachments_set,
            ambient_color: [0., 0., 0., 0.],
        }
    }

    pub fn recreate_pipeline(&mut self, context: &Renderer) {
        self.point_pipeline.recreate_pipeline(
            context.device.clone(),
            context.render_pass.clone(),
            context.viewport.clone(),
        );
        self.direction_pipeline.recreate_pipeline(
            context.device.clone(),
            context.render_pass.clone(),
            context.viewport.clone(),
        );
        self.ambient_pipeline.recreate_pipeline(
            context.device.clone(),
            context.render_pass.clone(),
            context.viewport.clone(),
        );
    }
    /// recreate the descriptor set describing the framebuffer attachments, must be done after recreating framebuffer (see `DrawSystem::recreate_pipelines`)
    pub fn recreate_descriptor(&mut self, context: &Renderer) {
        self.attachments_set = Self::create_attachment_set(&self.point_pipeline, context);
    }
    fn create_attachment_set(
        pipeline: &PipelineHandler<Vertex2d>,
        context: &Renderer,
    ) -> Arc<PersistentDescriptorSet> {
        PersistentDescriptorSet::new(
            &context.allocators.descriptor_set,
            pipeline.layout().set_layouts().get(0).unwrap().clone(),
            [
                WriteDescriptorSet::image_view(0, context.attachments.0.clone()),
                WriteDescriptorSet::image_view(1, context.attachments.1.clone()),
                WriteDescriptorSet::image_view(2, context.attachments.2.clone()),
            ],
            [],
        )
        .unwrap()
    }

    pub fn upload_lights(
        &mut self,
        point_lights: impl IntoIterator<Item = PointLight>,
        dir_lights: impl IntoIterator<Item = DirectionLight>,
        global_data: impl Into<GPUGlobalData>,
        ambient_color: impl Into<[f32; 4]>,
        image_i: usize,
    ) {
        self.frame_data[image_i].update(
            point_lights.into_iter(),
            dir_lights.into_iter(),
            global_data,
        );
        self.ambient_color = ambient_color.into();
    }

    pub fn render<P, A: vulkano::command_buffer::allocator::CommandBufferAllocator>(
        &mut self,
        image_i: usize,
        command_builder: &mut AutoCommandBufferBuilder<P, A>,
    ) {
        let frame = &self.frame_data[image_i];

        // println!(
        //     "Pipeline layout: {:?}",
        //     self.pipeline.layout().set_layouts()[1]
        // );
        // println!(
        //     "Set layout:      {:?}",
        //     frame.descriptor_sets[0].as_ref().0.layout()
        // );

        // bind commands
        // point lights
        let pipeline = &self.point_pipeline.pipeline;
        let layout = self.point_pipeline.layout();
        command_builder
            .bind_pipeline_graphics(pipeline.clone())
            .unwrap()
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                layout.clone(),
                0,
                self.attachments_set.clone(),
            )
            .unwrap()
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                layout.clone(),
                1,
                vec![frame.global_set.clone(), frame.point_set.clone()],
            )
            .unwrap()
            .bind_vertex_buffers(0, self.point_vertices.clone())
            .unwrap();
        for i in 0..=frame.last_point_index as u32 {
            command_builder
                .draw(self.point_vertices.len() as u32, 1, 0, i)
                .unwrap();
        }
        // directional lights
        let pipeline = &self.direction_pipeline.pipeline;
        let layout = self.direction_pipeline.layout();
        command_builder
            .bind_pipeline_graphics(pipeline.clone())
            .unwrap()
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                layout.clone(),
                0,
                self.attachments_set.clone(),
            )
            .unwrap()
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                layout.clone(),
                1,
                frame.dir_set.clone(),
            )
            .unwrap()
            .bind_vertex_buffers(0, self.screen_vertices.clone())
            .unwrap();
        for i in 0..=frame.last_dir_count as u32 {
            command_builder
                .draw(self.screen_vertices.len() as u32, 1, 0, i)
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
                self.attachments_set.clone(),
            )
            .unwrap()
            .push_constants(
                layout.clone(),
                0,
                lighting::GPUAmbientData {
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

struct FrameData {
    global_buffer: Subbuffer<GPUGlobalData>,
    point_buffer: Subbuffer<[PointLight]>,
    dir_buffer: Subbuffer<[DirectionLight]>,
    global_set: DescriptorSetWithOffsets,
    point_set: DescriptorSetWithOffsets,
    dir_set: DescriptorSetWithOffsets,
    last_point_index: usize,
    last_dir_count: usize,
}

impl FrameData {
    fn update(
        &mut self,
        point_lights: impl Iterator<Item = PointLight>,
        dir_lights: impl Iterator<Item = DirectionLight>,
        global_data: impl Into<GPUGlobalData>,
        // ambient_color: [f32; 4],
    ) {
        // point lights
        let mut contents = self
            .point_buffer
            .write()
            .unwrap_or_else(|e| panic!("Failed to write to point lights storage buffer\n{}", e));
        for (i, light) in point_lights.enumerate() {
            contents[i] = light;
            self.last_point_index = i;
        }

        // directional lights
        let mut contents = self.dir_buffer.write().unwrap_or_else(|e| {
            panic!(
                "Failed to write to directional lights storage buffer\n{}",
                e
            )
        });
        for (i, light) in dir_lights.enumerate() {
            contents[i] = light;
            self.last_dir_count = i;
        }

        // global
        let mut contents = self.global_buffer.write().unwrap_or_else(|e| {
            panic!(
                "Failed to write to directional lights storage buffer\n{}",
                e
            )
        });

        *contents = global_data.into();
    }
}
