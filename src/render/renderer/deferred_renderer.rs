use std::sync::Arc;

use super::{
    systems::{DrawSystem, LightingSystem},
    Renderer,
};
use crate::{
    render::{
        render_data::material::Shader,
        resource_manager::{ColoredID, ShaderID},
        Context,
    },
    shaders::{self, DirectionLight, GPUColoredData, GPUGlobalData, GPUObjectData, PointLight},
    vulkano_objects::{
        self,
        buffers::{write_to_buffer, write_to_storage_buffer, Uniform},
        pipeline::{mod_to_stages, LayoutOverrides},
        render_pass::FramebufferAttachments,
    },
};

use cgmath::Vector4;
use vulkano::{
    buffer::{Buffer, BufferCreateInfo, BufferUsage},
    command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer, RenderPassBeginInfo},
    device::Device,
    format::Format,
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter},
    render_pass::{Framebuffer, RenderPass, Subpass},
    shader::ShaderStages,
    swapchain::Swapchain,
};

/// 3D render that supports both lit and unlit meshes with deferred lighting
pub struct DeferredRenderer {
    /// Vulkan render pass for the framebuffer recreation
    render_pass: Arc<RenderPass>,
    /// for starting renderpass (deferred examples remakes fb's every frame)
    framebuffers: Vec<Arc<Framebuffer>>,
    /// misc attachments (depth, diffuse e.g)
    attachments: FramebufferAttachments,
    /// Data for rendering each frame
    pub frame_data: Vec<FrameData>,

    pub lit_draw_system: DrawSystem<ShaderID, ()>,
    pub unlit_draw_system: DrawSystem<ShaderID, ()>,

    pub lit_colored_system: DrawSystem<ColoredID, Vector4<f32>>,
    pub unlit_colored_system: DrawSystem<ColoredID, Vector4<f32>>,

    pub lighting_system: LightingSystem,
}
/// Stores the buffers and descriptor sets needed for rendering a frame
pub struct FrameData {
    global_data: Uniform<GPUGlobalData>,
    objects_data: Uniform<[GPUObjectData]>,
    colored_data: Uniform<[GPUColoredData]>,

    point_data: Uniform<[PointLight]>,
    last_point_index: Option<usize>,

    dir_data: Uniform<[DirectionLight]>,
    last_dir_index: Option<usize>,
}
// pub trait DataLoader<P, D>
// where
//     P: Iterator<Item = PointLight>,
//     D: Iterator<Item = DirectionLight>,
// {
//     fn get_global(&mut self) -> GPUGlobalData;
//     fn give_points<F>(&mut self, point_uploader: F)
//     where
//         F: FnOnce(&mut dyn Iterator<Item = PointLight>);
//     fn get_dir<F>(&mut self, dir_uploader: F)
//     where
//         F: FnOnce(impl Iterator<Item = DirectionLight>);
//     fn get_ambient(&mut self) -> [f32; 4];
// }

impl DeferredRenderer {
    pub fn new(context: &Context) -> Self {
        // let render_pass = deferred_render_pass(context.device.clone(), context.swapchain.clone());
        let render_pass =
            deferred_forward_render_pass(context.device.clone(), context.swapchain.clone());
        let (attachments, framebuffers) =
            vulkano_objects::render_pass::create_deferred_framebuffers_from_images(
                &context.images,
                render_pass.clone(),
                &context.allocators,
            );

        // create render systems
        let stages = mod_to_stages(
            context.device.clone(),
            shaders::load_basic_vs,
            shaders::load_basic_fs,
        );
        let colored_stages = mod_to_stages(
            context.device.clone(),
            shaders::load_new_colored_vs,
            shaders::load_new_solid_fs,
        );

        // global descriptor set layout
        let global_des_layout =
            LayoutOverrides::single_uniform_set(ShaderStages::VERTEX | ShaderStages::FRAGMENT);
        let layout_override = LayoutOverrides {
            set_layout_overrides: vec![(0, global_des_layout.clone())],
        };

        let lit_draw_system = DrawSystem::new(
            context,
            &Subpass::from(render_pass.clone(), 0).unwrap(),
            ShaderID::Texture,
            stages.clone(),
            layout_override.clone(),
        );
        let lit_colored_system = DrawSystem::new(
            context,
            &Subpass::from(render_pass.clone(), 0).unwrap(),
            ColoredID::Solid,
            colored_stages.clone(),
            layout_override.clone(),
        );

        let lighting_system = LightingSystem::new(
            context,
            &Subpass::from(render_pass.clone(), 1).unwrap(),
            &attachments,
            &layout_override,
        );

        let unlit_draw_system = DrawSystem::new(
            context,
            &Subpass::from(render_pass.clone(), 2).unwrap(),
            ShaderID::Texture,
            stages,
            layout_override.clone(),
        );
        let unlit_colored_system = DrawSystem::new(
            context,
            &Subpass::from(render_pass.clone(), 0).unwrap(),
            ColoredID::Solid,
            colored_stages.clone(),
            layout_override,
        );

        // create buffers and descriptor sets
        let image_count = context.get_image_count();

        // pack into frames
        let mut frame_data = vec![];
        for _ in 0..image_count {
            // shared global buffer
            let global_buffer = Buffer::from_data(
                context.allocators.memory.clone(),
                BufferCreateInfo {
                    usage: BufferUsage::UNIFORM_BUFFER,
                    ..Default::default()
                },
                AllocationCreateInfo {
                    memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                        | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                    ..Default::default()
                },
                Default::default(),
            )
            .unwrap();
            let global_set = lit_draw_system
                .first_shader()
                .pipeline
                .create_descriptor_set(
                    &context.allocators.descriptor_set,
                    global_buffer.clone(),
                    0,
                );

            // draw data
            let objects_data = lit_draw_system
                .first_shader()
                .pipeline
                .create_storage_buffer(&context.allocators, 1000, 1); //object_data.pop().unwrap();
            let colored_data = lit_colored_system
                .first_shader()
                .pipeline
                .create_storage_buffer(&context.allocators, 1000, 1);

            // lighting data
            let point_data =
                lighting_system
                    .point_pipeline
                    .create_storage_buffer(&context.allocators, 1000, 2);
            let dir_data = lighting_system.direction_pipeline.create_storage_buffer(
                &context.allocators,
                1000,
                2,
            );

            // println!("Creation layout: {:?}", global_set.as_ref().0.layout());

            frame_data.push(FrameData {
                global_data: (global_buffer, global_set),
                objects_data,
                colored_data,

                point_data,
                last_point_index: None,

                dir_data,
                last_dir_index: None,
            });
        }

        Self {
            render_pass,
            framebuffers,
            attachments,
            frame_data,

            lit_draw_system,
            unlit_draw_system,

            lighting_system,

            lit_colored_system,
            unlit_colored_system,
        }
    }

    // /// Uploads global, object and lighting data
    // pub fn update_frame_data<P, D>(&mut self, image_i: usize, mut data: impl DataLoader<P, D>)
    // where
    //     P: Iterator<Item = PointLight>,
    //     D: Iterator<Item = DirectionLight>,
    // {
    //     let frame = self
    //         .frame_data
    //         .get_mut(image_i)
    //         .expect("Renderer should have a frame for every swapchain image");

    //     // write_to_buffer(&frame.global_data.0, data.get_global());
    //     frame.update_global_data(data.get_global());

    //     frame.update_objects_data(
    //         self.lit_draw_system
    //             .shaders
    //             .values_mut()
    //             .chain(self.unlit_draw_system.shaders.values_mut()),
    //     );
    //     frame.update_colored_data(
    //         self.lit_colored_system
    //             .shaders
    //             .values_mut()
    //             .chain(self.unlit_colored_system.shaders.values_mut()),
    //     );

    //     // frame.last_point_index = write_to_storage_buffer(&frame.point_data.0, data.get_points(), 0);
    //     // frame.last_dir_index = write_to_storage_buffer(&frame.dir_data.0, data.get_dir(), 0);
    //     frame.update_point_lights(data.get_points());
    //     frame.update_directional_lights(data.get_dir());

    //     self.lighting_system.set_ambient_color(data.get_ambient());
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
        let mut object_index = 0;
        let mut colored_index = 0;

        // draw subpass
        self.lit_draw_system.render(
            &mut object_index,
            vec![frame.global_data.1.clone(), frame.objects_data.1.clone()],
            command_builder,
        );
        self.lit_colored_system.render(
            &mut colored_index,
            vec![frame.global_data.1.clone(), frame.colored_data.1.clone()],
            command_builder,
        );
        // end subpass
        command_builder
            .next_subpass(Default::default(), Default::default())
            .unwrap();

        // lighting subpass
        self.lighting_system.render(
            frame.global_data.1.clone().into(),
            frame.point_data.1.clone().into(),
            frame.dir_data.1.clone().into(),
            frame.last_point_index,
            frame.last_dir_index,
            command_builder,
        );
        // end subpass
        command_builder
            .next_subpass(Default::default(), Default::default())
            .unwrap();

        // unlit subpass
        self.unlit_draw_system.render(
            &mut object_index,
            vec![frame.global_data.1.clone(), frame.objects_data.1.clone()],
            command_builder,
        );
        self.unlit_colored_system.render(
            &mut colored_index,
            vec![frame.global_data.1.clone(), frame.colored_data.1.clone()],
            command_builder,
        );
        // end render pass
        command_builder.end_render_pass(Default::default()).unwrap();
    }

    fn recreate_pipelines(&mut self, context: &Context) {
        self.lit_draw_system.recreate_pipelines(context);
        self.lighting_system.recreate_pipeline(context);
        self.unlit_draw_system.recreate_pipelines(context);
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

impl FrameData {
    /// write global data to buffer
    pub fn update_global_data(&mut self, data: impl Into<GPUGlobalData>) {
        write_to_buffer(&self.global_data.0, data);
    }

    /// write object data to storage buffer
    ///
    /// `RenderObject::upload(&self)` must have been called beforehand
    pub fn update_objects_data<'a>(&self, shaders: impl Iterator<Item = &'a mut Shader<()>>) {
        let obj_iter = shaders.flat_map(|pipeline| pipeline.upload_pending_objects());
        write_to_storage_buffer(&self.objects_data.0, obj_iter, 0);
    }
    /// write colored data to storage buffer
    ///
    /// `RenderObject::upload(&self)` must have been called beforehand
    pub fn update_colored_data<'a>(
        &self,
        shaders: impl Iterator<Item = &'a mut Shader<Vector4<f32>>>,
    ) {
        let obj_iter = shaders.flat_map(|pipeline| pipeline.upload_pending_objects());
        write_to_storage_buffer(&self.colored_data.0, obj_iter, 0);
    }

    pub fn update_point_lights(&mut self, point_lights: impl Iterator<Item = PointLight>) {
        self.last_point_index = write_to_storage_buffer(&self.point_data.0, point_lights, 0);
    }
    pub fn update_directional_lights(&mut self, dir_lights: impl Iterator<Item = DirectionLight>) {
        self.last_dir_index = write_to_storage_buffer(&self.dir_data.0, dir_lights, 0);
    }
}

/// Creates render pass with 2 subpasses and diffuse, normal and depth attachments for deferred shading
#[allow(dead_code)]
fn deferred_render_pass(device: Arc<Device>, swapchain: Arc<Swapchain>) -> Arc<RenderPass> {
    vulkano::ordered_passes_renderpass!(
        device,
    attachments: {
            // The image that will contain the final rendering (in this example the swapchain
            // image, but it could be another image).
            final_color: {
                format: swapchain.image_format(),
                samples: 1,
                load_op: Clear,
                store_op: Store,
            },
            // Diffuse buffer (unlit color)
            diffuse: {
                format: Format::A2B10G10R10_UNORM_PACK32,
                samples: 1,
                load_op: Clear,
                store_op: DontCare,
            },
            // Normal buffer
            normals: {
                format: Format::R16G16B16A16_SFLOAT,
                samples: 1,
                load_op: Clear,
                store_op: DontCare,
            },
            // Depth buffer
            depth_stencil: {
                format: Format::D32_SFLOAT,
                samples: 1,
                load_op: Clear,
                store_op: DontCare,
            },
        },
        passes: [
            // Write to the diffuse, normals and depth attachments.
            {
                color: [diffuse, normals],
                depth_stencil: {depth_stencil},
                input: [],
            },
            // Apply lighting by reading these three attachments and writing to `final_color`.
            {
                color: [final_color],
                depth_stencil: {},
                input: [diffuse, normals, depth_stencil],
            },
        ],
    )
    .unwrap()
}

/// Creates render pass with 2 subpasses and diffuse, normal and depth attachments for deferred shading and an additional subpass for forward rendering
fn deferred_forward_render_pass(device: Arc<Device>, swapchain: Arc<Swapchain>) -> Arc<RenderPass> {
    vulkano::ordered_passes_renderpass!(
        device,
    attachments: {
            // The image that will contain the final rendering (in this example the swapchain
            // image, but it could be another image).
            final_color: {
                format: swapchain.image_format(),
                samples: 1,
                load_op: Clear,
                store_op: Store,
            },
            // Diffuse buffer (unlit color)
            diffuse: {
                format: Format::A2B10G10R10_UNORM_PACK32,
                samples: 1,
                load_op: Clear,
                store_op: DontCare,
            },
            // Normal buffer
            normals: {
                format: Format::R16G16B16A16_SFLOAT,
                samples: 1,
                load_op: Clear,
                store_op: DontCare,
            },
            // Depth buffer
            depth_stencil: {
                format: Format::D32_SFLOAT,
                samples: 1,
                load_op: Clear,
                store_op: DontCare,
            },
        },
        passes: [
            // Write to the diffuse, normals and depth attachments.
            {
                color: [diffuse, normals],
                depth_stencil: {depth_stencil},
                input: [],
            },
            // Apply lighting by reading these three attachments and writing to `final_color`.
            {
                color: [final_color],
                depth_stencil: {},
                input: [diffuse, normals, depth_stencil],
            },
            // forward renderpass
            {
                color: [final_color, normals],
                depth_stencil: {depth_stencil},
                input: [],
            },
        ],
    )
    .unwrap()
}
