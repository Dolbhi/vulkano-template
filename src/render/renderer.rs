use std::{path::Path, sync::Arc};

use crate::{
    vulkano_objects::{
        self, allocators::Allocators, buffers::Buffers, render_pass::FramebufferAttachments,
    },
    VertexFull,
};
use vulkano::{
    buffer::BufferContents,
    command_buffer::{self, RenderPassBeginInfo},
    descriptor_set::PersistentDescriptorSet,
    device::{Device, DeviceCreateInfo, DeviceExtensions, Queue, QueueCreateInfo},
    image::{sampler::Sampler, view::ImageView},
    instance::Instance,
    pipeline::graphics::viewport::Viewport,
    render_pass::{Framebuffer, RenderPass},
    swapchain::{
        self, PresentFuture, Surface, Swapchain, SwapchainAcquireFuture, SwapchainCreateInfo,
        SwapchainPresentInfo,
    },
    sync::{
        self,
        future::{FenceSignalFuture, JoinFuture, NowFuture},
        GpuFuture,
    },
    Validated, VulkanError,
};
use winit::{
    dpi::LogicalSize,
    event_loop::EventLoop,
    window::{CursorGrabMode, Window, WindowBuilder},
};

use super::{
    lighting_system::LightingSystem,
    render_data::{
        material::PipelineGroup,
        texture::{create_sampler, load_texture},
    },
    DrawSystem,
};

pub type Fence = FenceSignalFuture<
    PresentFuture<
        command_buffer::CommandBufferExecFuture<
            JoinFuture<Box<dyn GpuFuture>, SwapchainAcquireFuture>,
        >,
    >,
>;

const INIT_WINDOW_SIZE: LogicalSize<f32> = LogicalSize::new(1000.0f32, 600.0);

pub struct Renderer {
    _instance: Arc<Instance>,
    pub window: Arc<Window>, // for get inner size and request redraw
    pub viewport: Viewport,  // just for pipeline creation
    pub device: Arc<Device>,
    pub queue: Arc<Queue>, // for submitting command buffers
    pub allocators: Allocators,
    swapchain: Arc<Swapchain>, // swapchain recreation and image presenting
    pub render_pass: Arc<RenderPass>,
    framebuffers: Vec<Arc<Framebuffer>>, // for starting renderpass (deferred examples remakes fb's every frame)
    pub attachments: FramebufferAttachments, // misc attachments (depth, diffuse e.g)
}

impl Renderer {
    pub fn initialize(event_loop: &EventLoop<()>) -> Self {
        let instance = vulkano_objects::instance::get_instance(event_loop);

        let window = Arc::new(WindowBuilder::new().build(&event_loop).unwrap());
        let surface = Surface::from_window(instance.clone(), window.clone()).unwrap();

        // window settings
        window.set_title("Rusty Renderer");
        let _new_size = window.request_inner_size(INIT_WINDOW_SIZE);
        window.set_cursor_visible(false);
        window
            .set_cursor_grab(CursorGrabMode::Confined)
            .or_else(|_e| window.set_cursor_grab(CursorGrabMode::Locked))
            .unwrap();

        let viewport: Viewport = Viewport {
            extent: window.inner_size().into(),
            ..Default::default() // offset: [0.0, 0.0],
                                 // depth_range: 0.0..=1.0,
        };

        let device_extensions = DeviceExtensions {
            khr_swapchain: true,
            khr_shader_draw_parameters: true,
            ..DeviceExtensions::empty()
        };
        let (physical_device, queue_family_index) =
            vulkano_objects::physical_device::select_physical_device(
                &instance,
                surface.clone(),
                &device_extensions,
            );

        let (device, mut queues) = Device::new(
            physical_device.clone(),
            DeviceCreateInfo {
                queue_create_infos: vec![QueueCreateInfo {
                    queue_family_index,
                    ..Default::default()
                }],
                enabled_extensions: device_extensions, // new
                ..Default::default()
            },
        )
        .expect("failed to create device");

        let allocators = Allocators::new(device.clone());

        let queue = queues.next().unwrap();

        let (swapchain, images) =
            vulkano_objects::swapchain::create_swapchain(&physical_device, device.clone(), surface);

        let render_pass = vulkano_objects::render_pass::create_deferred_render_pass(
            device.clone(),
            swapchain.clone(),
        );
        let (attachments, framebuffers) =
            vulkano_objects::render_pass::create_deferred_framebuffers_from_images(
                &images,
                render_pass.clone(),
                &allocators,
            );

        println!(
            "[Renderer info]\nswapchain image count: {}\nQueue family: {}\nrgba format properties: {:?}",
            images.len(),
            queue_family_index,
            physical_device
                .format_properties(vulkano::format::Format::R8G8B8A8_SRGB)
                .unwrap()
                .optimal_tiling_features,
        );

        Self {
            _instance: instance,
            window,
            device,
            queue,
            allocators,
            swapchain,
            render_pass,
            framebuffers,
            attachments,
            viewport,
        }
    }

    pub fn get_image_count(&self) -> usize {
        self.swapchain.image_count() as usize
    }

    /// recreates swapchain and framebuffers
    pub fn recreate_swapchain(&mut self) {
        let (new_swapchain, new_images) = match self.swapchain.recreate(SwapchainCreateInfo {
            image_extent: self.window.inner_size().into(),
            ..self.swapchain.create_info()
        }) {
            Ok(r) => r,
            // Err(SwapchainCreationError::ImageExtentNotSupported { .. }) => return,
            Err(e) => panic!("Failed to recreate swapchain: {:?}", e),
        };

        self.swapchain = new_swapchain;
        (self.attachments, self.framebuffers) =
            vulkano_objects::render_pass::create_deferred_framebuffers_from_images(
                &new_images,
                self.render_pass.clone(),
                &self.allocators,
            );
    }

    /// Gets future where next image in swapchain is ready
    pub fn acquire_swapchain_image(
        &self,
    ) -> Result<(u32, bool, SwapchainAcquireFuture), Validated<VulkanError>> {
        swapchain::acquire_next_image(self.swapchain.clone(), None)
    }

    pub fn synchronize(&self) -> NowFuture {
        let mut now = sync::now(self.device.clone());
        now.cleanup_finished();

        now
    }

    /// Join given futures then execute new commands and present the swapchain image corresponding to the given image_i
    pub fn flush_next_future<O, T>(
        &self,
        previous_future: Box<dyn GpuFuture>,
        swapchain_acquire_future: SwapchainAcquireFuture,
        image_i: u32,
        draw_system: &mut DrawSystem<O, T>,
        lighting_system: &mut LightingSystem,
    ) -> Result<Fence, Validated<VulkanError>>
    where
        O: BufferContents + From<T>,
        T: Clone,
    {
        let index = image_i as usize;

        // create builder
        let mut builder = command_buffer::AutoCommandBufferBuilder::primary(
            &self.allocators.command_buffer,
            self.queue.queue_family_index(),
            command_buffer::CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        // start render pass
        builder
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
        draw_system.render(index, &mut builder);
        // end subpass
        builder
            .next_subpass(Default::default(), Default::default())
            .unwrap();
        // lighting pass
        lighting_system.render(index, &mut builder);
        // end render pass
        builder.end_render_pass(Default::default()).unwrap();

        // Join given futures then execute new commands and present the swapchain image corresponding to the given image_i
        previous_future
            .join(swapchain_acquire_future)
            .then_execute(self.queue.clone(), builder.build().unwrap())
            .unwrap()
            .then_swapchain_present(
                self.queue.clone(),
                SwapchainPresentInfo::swapchain_image_index(self.swapchain.clone(), image_i),
            )
            .then_signal_fence_and_flush()
    }

    pub fn get_resource_loader(&self) -> ResourceLoader {
        ResourceLoader { context: &self }
    }
}

pub struct ResourceLoader<'a> {
    context: &'a Renderer,
    // draw_system: &'a mut DrawSystem<GPUObjectData, Matrix4<f32>>,
}

impl<'a> ResourceLoader<'a> {
    pub fn load_texture(&self, path: &Path) -> Arc<ImageView> {
        load_texture(&self.context.allocators, &self.context.queue, path)
    }
    pub fn load_sampler(&self, filer: vulkano::image::sampler::Filter) -> Arc<Sampler> {
        create_sampler(self.context.device.clone(), filer)
    }
    pub fn load_mesh(
        &self,
        vertices: Vec<VertexFull>,
        indices: Vec<u32>,
    ) -> Arc<Buffers<VertexFull>> {
        Arc::new(Buffers::initialize_device_local(
            &self.context.allocators,
            self.context.queue.clone(),
            vertices,
            indices,
        ))
    }
    /// creates a texture sampler material set with the 3rd descriptor set layout of given pipeline
    pub fn load_material_set(
        &self,
        pipeline_group: &PipelineGroup,
        texture: Arc<ImageView>,
        sampler: Arc<Sampler>,
    ) -> Arc<PersistentDescriptorSet> {
        pipeline_group.create_material_set(&self.context.allocators, 2, texture, sampler)
    }
    // pub fn build_material
    // pub fn
}
