use std::{iter::zip, sync::Arc};

use super::Renderer;
use crate::{
    render::{lighting_system::LightingSystem, Context, DrawSystem, RenderObject},
    shaders::draw::{self, GPUGlobalData, GPUObjectData},
    vulkano_objects::{
        self,
        buffers::{create_dynamic_buffers, create_storage_buffers, write_to_buffer},
        render_pass::FramebufferAttachments,
    },
};

use cgmath::Matrix4;
use vulkano::{
    buffer::Subbuffer,
    command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer, RenderPassBeginInfo},
    descriptor_set::DescriptorSetWithOffsets,
    render_pass::{Framebuffer, RenderPass},
};

pub struct DeferredRenderer {
    render_pass: Arc<RenderPass>,
    framebuffers: Vec<Arc<Framebuffer>>, // for starting renderpass (deferred examples remakes fb's every frame)
    attachments: FramebufferAttachments, // misc attachments (depth, diffuse e.g)
    pub frame_data: Vec<FrameData>,
    pub draw_system: DrawSystem<Matrix4<f32>>,
    pub lighting_system: LightingSystem,
}

impl DeferredRenderer {
    pub fn new(context: &Context) -> Self {
        let render_pass = vulkano_objects::render_pass::create_deferred_render_pass(
            context.device.clone(),
            context.swapchain.clone(),
        );
        let (attachments, framebuffers) =
            vulkano_objects::render_pass::create_deferred_framebuffers_from_images(
                &context.images,
                render_pass.clone(),
                &context.allocators,
            );

        let (draw_system, [global_layout, objects_layout]) = {
            let shaders = [
                (
                    draw::load_basic_vs(context.device.clone())
                        .expect("failed to create basic shader module"),
                    draw::load_basic_fs(context.device.clone())
                        .expect("failed to create basic shader module"),
                ),
                (
                    draw::load_basic_vs(context.device.clone())
                        .expect("failed to create uv shader module"),
                    draw::load_uv_fs(context.device.clone())
                        .expect("failed to create uv shader module"),
                ),
            ];

            DrawSystem::new(
                &context,
                &render_pass,
                shaders.map(|(v, f)| {
                    (
                        v.entry_point("main").unwrap(),
                        f.entry_point("main").unwrap(),
                    )
                }),
            )
        };
        let lighting_system = LightingSystem::new(&context, &render_pass, &attachments);

        // create buffers and descriptor sets
        let image_count = context.get_image_count();
        let global_data = create_dynamic_buffers::<GPUGlobalData>(
            &context.allocators,
            &context.device,
            global_layout,
            image_count,
        );
        let object_data =
            create_storage_buffers(&context.allocators, objects_layout, image_count, 10000);

        // create frame data
        let frame_data = zip(global_data, object_data)
            .map(
                |((global_buffer, global_set), (objects_buffer, objects_set))| FrameData {
                    global_buffer,
                    objects_buffer,
                    global_set,
                    objects_set: objects_set.into(),
                },
            )
            .collect();

        Self {
            render_pass,
            framebuffers,
            attachments,
            frame_data,
            draw_system,
            lighting_system,
        }
    }

    // pub fn get_frame_mut(&mut self, index: usize) -> Option<&mut FrameData> {
    //     self.frame_data.get_mut(index)
    // }
}
impl Renderer for DeferredRenderer {
    fn build_command_buffer(
        &mut self,
        index: usize,
        command_builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    ) {
        // start render pass
        command_builder
            .begin_render_pass(
                RenderPassBeginInfo {
                    clear_values: vec![
                        Some([0.0, 0.0, 0.0, 0.0].into()), // swapchain image
                        Some([0.0, 0.0, 0.0, 0.0].into()), // diffuse buffer
                        Some([0.0, 0.0, 0.0, 0.0].into()), // normal buffer
                        Some(1.0f32.into()),               // depth buffer
                    ],
                    ..RenderPassBeginInfo::framebuffer(self.framebuffers[index].clone())
                },
                Default::default(),
            )
            .unwrap();

        // get frame data
        let frame = &self.frame_data[index];

        // draw pass
        self.draw_system.render(
            frame.global_set.clone(),
            frame.objects_set.clone(),
            command_builder,
        );
        // end subpass
        command_builder
            .next_subpass(Default::default(), Default::default())
            .unwrap();
        // lighting pass
        self.lighting_system.render(index, command_builder);
        // end render pass
        command_builder.end_render_pass(Default::default()).unwrap();
    }

    fn recreate_pipelines(&mut self, context: &Context) {
        self.draw_system
            .recreate_pipelines(context, &self.render_pass);
        self.lighting_system
            .recreate_pipeline(context, &self.render_pass);
    }
    fn recreate_framebuffers(&mut self, context: &Context) {
        (self.attachments, self.framebuffers) =
            vulkano_objects::render_pass::create_deferred_framebuffers_from_images(
                &context.images,
                self.render_pass.clone(),
                &context.allocators,
            );
        self.lighting_system
            .recreate_descriptor(context, &self.attachments);
    }
}

pub struct FrameData {
    global_buffer: Subbuffer<GPUGlobalData>,
    objects_buffer: Subbuffer<[GPUObjectData]>,

    global_set: DescriptorSetWithOffsets,
    objects_set: DescriptorSetWithOffsets,
}

impl FrameData {
    pub fn update_global_data(&mut self, data: impl Into<GPUGlobalData>) {
        write_to_buffer(&self.global_buffer, data);
    }

    pub fn update_objects_data<'a>(
        &self,
        render_objects: impl Iterator<Item = &'a Arc<RenderObject<Matrix4<f32>>>>,
        draw_system: &mut DrawSystem<Matrix4<f32>>,
    ) {
        draw_system.upload_object_data(render_objects, &self.objects_buffer)
    }
}
