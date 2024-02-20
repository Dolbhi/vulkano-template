use std::sync::Arc;

use super::{
    systems::{DrawSystem, LightingSystem},
    Renderer,
};
use crate::{
    render::{Context, RenderObject},
    shaders::{
        draw::{self, GPUGlobalData, GPUObjectData},
        lighting::{DirectionLight, PointLight},
    },
    vulkano_objects::{
        self,
        buffers::{create_storage_buffers, write_to_buffer, write_to_storage_buffer},
        render_pass::FramebufferAttachments,
    },
};

use cgmath::Matrix4;
use vulkano::{
    buffer::{Buffer, BufferCreateInfo, BufferUsage, Subbuffer},
    command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer, RenderPassBeginInfo},
    descriptor_set::{DescriptorSetWithOffsets, PersistentDescriptorSet, WriteDescriptorSet},
    device::Device,
    format::Format,
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter},
    render_pass::{Framebuffer, RenderPass, Subpass},
    swapchain::Swapchain,
};

pub struct DeferredRenderer {
    render_pass: Arc<RenderPass>,
    framebuffers: Vec<Arc<Framebuffer>>, // for starting renderpass (deferred examples remakes fb's every frame)
    attachments: FramebufferAttachments, // misc attachments (depth, diffuse e.g)
    pub frame_data: Vec<FrameData>,
    pub lit_draw_system: DrawSystem,
    pub lighting_system: LightingSystem,
    pub unlit_draw_system: DrawSystem,
}

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

        let (lit_draw_system, [global_draw_layout, objects_layout]) = {
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
                &Subpass::from(render_pass.clone(), 0).unwrap(),
                shaders.map(|(v, f)| {
                    (
                        v.entry_point("main").unwrap(),
                        f.entry_point("main").unwrap(),
                    )
                }),
            )
        };
        let (lighting_system, [global_light_layout, point_layout, dir_layout]) =
            LightingSystem::new(
                &context,
                &Subpass::from(render_pass.clone(), 1).unwrap(),
                &attachments,
            );
        let (unlit_draw_system, [_, unlit_objects_layout]) = {
            let shaders = [
                (
                    draw::load_basic_vs(context.device.clone())
                        .expect("failed to create basic shader module"),
                    draw::load_solid_fs(context.device.clone())
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
                &Subpass::from(render_pass.clone(), 2).unwrap(),
                shaders.map(|(v, f)| {
                    (
                        v.entry_point("main").unwrap(),
                        f.entry_point("main").unwrap(),
                    )
                }),
            )
        };

        // create buffers and descriptor sets
        let image_count = context.get_image_count();
        // let mut global_data = create_dynamic_buffers::<GPUGlobalData>(
        //     &context.allocators,
        //     &context.device,
        //     global_layout,
        //     image_count,
        // );
        let mut object_data =
            create_storage_buffers(&context.allocators, objects_layout, image_count, 10000);
        let mut unlit_data = create_storage_buffers(
            &context.allocators,
            unlit_objects_layout,
            image_count,
            10000,
        );

        // create frame data
        let mut point_data = create_storage_buffers::<PointLight>(
            &context.allocators,
            point_layout,
            image_count,
            1000,
        );
        let mut dir_data = create_storage_buffers::<DirectionLight>(
            &context.allocators,
            dir_layout,
            image_count,
            1000,
        );

        // pack into frames
        let mut frame_data = vec![];
        for _ in 0..image_count {
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
            // let (global_buffer, global_draw_set) = global_data.pop().unwrap();

            let global_draw_set = PersistentDescriptorSet::new(
                &context.allocators.descriptor_set,
                global_draw_layout.clone(),
                [WriteDescriptorSet::buffer(0, global_buffer.clone())],
                [],
            )
            .unwrap()
            .into();
            let (objects_buffer, objects_set) = object_data.pop().unwrap();
            let (unlit_buffer, unlit_set) = unlit_data.pop().unwrap();

            let global_light_set = PersistentDescriptorSet::new(
                &context.allocators.descriptor_set,
                global_light_layout.clone(),
                [WriteDescriptorSet::buffer(0, global_buffer.clone())],
                [],
            )
            .unwrap()
            .into();
            let (point_buffer, point_set) = point_data.pop().unwrap();
            let (dir_buffer, dir_set) = dir_data.pop().unwrap();

            // println!("Creation layout: {:?}", global_set.as_ref().0.layout());

            frame_data.push(FrameData {
                global_buffer,
                objects_buffer,
                unlit_buffer,
                point_buffer,
                dir_buffer,

                global_draw_set,
                objects_set: objects_set.into(),
                unlit_set: unlit_set.into(),
                point_set: point_set.into(),
                global_light_set,
                dir_set: dir_set.into(),
                last_point_index: None,
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

        // draw subpass
        self.lit_draw_system.render(
            vec![frame.global_draw_set.clone(), frame.objects_set.clone()],
            command_builder,
        );
        // end subpass
        command_builder
            .next_subpass(Default::default(), Default::default())
            .unwrap();

        // lighting subpass
        self.lighting_system.render(
            frame.global_light_set.clone(),
            frame.point_set.clone(),
            frame.dir_set.clone(),
            frame.last_point_index,
            frame.last_point_index,
            command_builder,
        );
        // end subpass
        command_builder
            .next_subpass(Default::default(), Default::default())
            .unwrap();

        // unlit subpass
        self.unlit_draw_system.render(
            vec![frame.global_draw_set.clone(), frame.unlit_set.clone()],
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

pub struct FrameData {
    global_buffer: Subbuffer<GPUGlobalData>,
    objects_buffer: Subbuffer<[GPUObjectData]>,
    unlit_buffer: Subbuffer<[GPUObjectData]>,
    point_buffer: Subbuffer<[PointLight]>,
    dir_buffer: Subbuffer<[DirectionLight]>,

    global_draw_set: DescriptorSetWithOffsets,
    objects_set: DescriptorSetWithOffsets,
    unlit_set: DescriptorSetWithOffsets,
    global_light_set: DescriptorSetWithOffsets,
    point_set: DescriptorSetWithOffsets,
    dir_set: DescriptorSetWithOffsets,
    last_point_index: Option<usize>,
    last_dir_index: Option<usize>,
}

impl FrameData {
    pub fn update_global_data(&mut self, data: impl Into<GPUGlobalData>) {
        write_to_buffer(&self.global_buffer, data);
    }

    pub fn update_objects_data<'a>(&self, draw_system: &mut DrawSystem) {
        draw_system.update_object_buffer(&self.objects_buffer);
    }
    pub fn update_unlit_data<'a>(&self, draw_system: &mut DrawSystem) {
        draw_system.update_object_buffer(&self.unlit_buffer);
    }

    pub fn update_point_lights(&mut self, point_lights: impl Iterator<Item = PointLight>) {
        self.last_point_index = write_to_storage_buffer(&self.point_buffer, point_lights);
    }
    pub fn update_directional_lights(&mut self, dir_lights: impl Iterator<Item = DirectionLight>) {
        self.last_dir_index = write_to_storage_buffer(&self.dir_buffer, dir_lights);
    }
}

/// Creates render pass with 2 subpasses and diffuse, normal and depth attachments for deferred shading
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
