use std::sync::Arc;

use super::Renderer;
use crate::{
    render::{lighting_system::LightingSystem, Context, DrawSystem, RenderObject},
    shaders::{
        draw::{self, GPUGlobalData, GPUObjectData},
        lighting::{DirectionLight, PointLight},
    },
    vulkano_objects::{self, render_pass::FramebufferAttachments},
};

use cgmath::Matrix4;
use vulkano::{
    command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer, RenderPassBeginInfo},
    render_pass::{Framebuffer, RenderPass},
};

pub struct DeferredRenderer {
    render_pass: Arc<RenderPass>,
    framebuffers: Vec<Arc<Framebuffer>>, // for starting renderpass (deferred examples remakes fb's every frame)
    attachments: FramebufferAttachments, // misc attachments (depth, diffuse e.g)
    pub draw_system: DrawSystem<GPUObjectData, Matrix4<f32>>,
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

        let draw_system = {
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

        Self {
            render_pass,
            framebuffers,
            attachments,
            draw_system,
            lighting_system,
        }
    }

    pub fn upload_data<'a>(
        &mut self,
        image_i: usize,
        global_data: GPUGlobalData,
        render_objects: impl Iterator<Item = &'a Arc<RenderObject<Matrix4<f32>>>>,
        point_lights: impl IntoIterator<Item = PointLight>,
        dir_lights: impl IntoIterator<Item = DirectionLight>,
        ambient_color: impl Into<[f32; 4]>,
    ) {
        self.draw_system
            .upload_draw_data(image_i, render_objects, global_data);

        // println!("Projection view:");
        // let matrix: [[f32; 4]; 4] = proj_view.clone().into();
        // for x in matrix {
        //     println!("{:11}, {:11}, {:11}, {:11},", x[0], x[1], x[2], x[3]);
        // }

        self.lighting_system.upload_lights(
            point_lights,
            dir_lights,
            ambient_color,
            global_data,
            image_i,
        );
    }
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

        // draw pass
        self.draw_system.render(index, command_builder);
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
