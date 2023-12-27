use std::sync::Arc;

use vulkano::{
    buffer::{BufferUsage, Subbuffer},
    command_buffer::AutoCommandBufferBuilder,
    descriptor_set::{DescriptorSetWithOffsets, PersistentDescriptorSet, WriteDescriptorSet},
    pipeline::PipelineBindPoint,
    sync::GpuFuture,
};

use crate::{
    shaders::lighting::{
        self,
        fs::{DirectionLight, GPULightingData, PointLight},
    },
    vulkano_objects::{
        buffers::{create_device_local_buffer, create_global_descriptors, create_storage_buffers},
        pipeline::PipelineHandler,
    },
    Vertex2d,
};

use super::Renderer;

pub struct LightingSystem {
    pipeline: PipelineHandler<Vertex2d>,
    frame_data: Vec<FrameData>,
    vertex_buffer: Subbuffer<[Vertex2d]>,
    attachments_set: Arc<PersistentDescriptorSet>,
}

impl LightingSystem {
    pub fn new(context: &Renderer) -> Self {
        // create pipeline
        let pipeline = {
            let vs = lighting::vs::load(context.device.clone())
                .expect("failed to create lighting shader module")
                .entry_point("main")
                .unwrap();
            let fs = lighting::fs::load(context.device.clone())
                .expect("failed to create lighting shader module")
                .entry_point("main")
                .unwrap();
            PipelineHandler::new(
                context.device.clone(),
                vs,
                fs,
                context.viewport.clone(),
                context.render_pass.clone(),
                [(1, 0)],
                crate::vulkano_objects::pipeline::PipelineType::Lighting,
            )
        };

        let layout = pipeline.layout();
        let image_count = context.swapchain.image_count() as usize;

        // create buffers and descriptor sets
        let attachments_set = PersistentDescriptorSet::new(
            &context.allocators.descriptor_set,
            layout.set_layouts().get(0).unwrap().clone(),
            [
                WriteDescriptorSet::image_view(0, context.attachments.0.clone()),
                WriteDescriptorSet::image_view(1, context.attachments.1.clone()),
                WriteDescriptorSet::image_view(2, context.attachments.2.clone()),
            ],
            [],
        )
        .unwrap()
        .into();

        let mut global_data = create_global_descriptors::<GPULightingData>(
            &context.allocators,
            &context.device,
            layout.set_layouts().get(1).unwrap().clone(),
            image_count,
        );
        let mut point_data = create_storage_buffers::<PointLight>(
            &context.allocators,
            layout.set_layouts().get(2).unwrap().clone(),
            image_count,
            1000,
        );
        let mut dir_data = create_storage_buffers::<DirectionLight>(
            &context.allocators,
            layout.set_layouts().get(3).unwrap().clone(),
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
                descriptor_sets: vec![global_set, point_set.into(), dir_set.into()],
            });
        }

        let (vertex_buffer, vertex_future) = create_device_local_buffer(
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
        // let (point_vertices, point_future) = create_device_local_buffer(
        //     &context.allocators,
        //     context.queue.clone(),
        //     vec![
        //         Vertex2d {
        //             position: [-1.0, -1.0],
        //         },
        //         Vertex2d {
        //             position: [-1.0, 1.0],
        //         },
        //         Vertex2d {
        //             position: [1.0, -1.0],
        //         },
        //         Vertex2d {
        //             position: [1.0, -1.0],
        //         },
        //         Vertex2d {
        //             position: [-1.0, 1.0],
        //         },
        //         Vertex2d {
        //             position: [1.0, 1.0],
        //         },
        //     ],
        //     BufferUsage::VERTEX_BUFFER,
        // );

        let fence = vertex_future.then_signal_fence_and_flush().unwrap();
        fence.wait(None).unwrap();

        LightingSystem {
            pipeline,
            frame_data,
            vertex_buffer,
            attachments_set,
        }
    }

    pub fn recreate_pipeline(&mut self, context: &Renderer) {
        self.pipeline.recreate_pipeline(
            context.device.clone(),
            context.render_pass.clone(),
            context.viewport.clone(),
        );
    }
    /// recreate the descriptor set describing the framebuffer attachments, must be done after recreating framebuffer (see `DrawSystem::recreate_pipelines`)
    pub fn recreate_descriptor(&mut self, context: &Renderer) {
        let attachments = PersistentDescriptorSet::new(
            &context.allocators.descriptor_set,
            self.pipeline.layout().set_layouts().get(0).unwrap().clone(),
            [
                WriteDescriptorSet::image_view(0, context.attachments.0.clone()),
                WriteDescriptorSet::image_view(1, context.attachments.1.clone()),
                WriteDescriptorSet::image_view(2, context.attachments.2.clone()),
            ],
            [],
        )
        .unwrap();

        self.attachments_set = attachments;
    }

    pub fn upload_lights(
        &self,
        point_lights: impl IntoIterator<Item = PointLight>,
        dir_lights: impl IntoIterator<Item = DirectionLight>,
        screen_to_world: impl Into<[[f32; 4]; 4]>,
        ambient_color: impl Into<[f32; 4]>,
        image_i: usize,
    ) {
        self.frame_data[image_i].update(
            point_lights.into_iter(),
            dir_lights.into_iter(),
            screen_to_world.into(),
            ambient_color.into(),
        );
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
        command_builder
            .bind_pipeline_graphics(self.pipeline.pipeline.clone())
            .unwrap()
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                self.pipeline.layout().clone(),
                0,
                self.attachments_set.clone(),
            )
            .unwrap()
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                self.pipeline.layout().clone(),
                1,
                frame.descriptor_sets.clone(),
            )
            .unwrap()
            .bind_vertex_buffers(0, self.vertex_buffer.clone())
            .unwrap()
            .draw(self.vertex_buffer.len() as u32, 1, 0, 0)
            .unwrap();
    }
}

struct FrameData {
    global_buffer: Subbuffer<GPULightingData>,
    point_buffer: Subbuffer<[PointLight]>,
    dir_buffer: Subbuffer<[DirectionLight]>,
    descriptor_sets: Vec<DescriptorSetWithOffsets>,
}

impl FrameData {
    fn update(
        &self,
        point_lights: impl Iterator<Item = PointLight>,
        dir_lights: impl Iterator<Item = DirectionLight>,
        screen_to_world: [[f32; 4]; 4],
        ambient_color: [f32; 4],
    ) {
        // point lights
        let mut point_light_count = 0;
        let mut contents = self
            .point_buffer
            .write()
            .unwrap_or_else(|e| panic!("Failed to write to point lights storage buffer\n{}", e));

        for (i, light) in point_lights.enumerate() {
            contents[i] = light;
            point_light_count = i;
        }
        point_light_count += 1;
        // directional lights
        let mut direction_light_count = 0;
        let mut contents = self.dir_buffer.write().unwrap_or_else(|e| {
            panic!(
                "Failed to write to directional lights storage buffer\n{}",
                e
            )
        });

        for (i, light) in dir_lights.enumerate() {
            contents[i] = light;
            direction_light_count = i;
        }
        direction_light_count += 1;

        // global
        let mut contents = self.global_buffer.write().unwrap_or_else(|e| {
            panic!(
                "Failed to write to directional lights storage buffer\n{}",
                e
            )
        });

        *contents = GPULightingData {
            screen_to_world,
            ambient_color,
            point_light_count: point_light_count as u32,
            direction_light_count: direction_light_count as u32,
        };
    }
}
