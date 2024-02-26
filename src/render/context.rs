use std::sync::Arc;

use crate::vulkano_objects::{self, allocators::Allocators};
use vulkano::{
    command_buffer::{self, AutoCommandBufferBuilder, PrimaryAutoCommandBuffer},
    device::{Device, DeviceCreateInfo, DeviceExtensions, Queue, QueueCreateInfo},
    image::Image,
    instance::Instance,
    pipeline::graphics::viewport::Viewport,
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

pub type Fence = FenceSignalFuture<
    PresentFuture<
        command_buffer::CommandBufferExecFuture<
            JoinFuture<Box<dyn GpuFuture>, SwapchainAcquireFuture>,
        >,
    >,
>;

const INIT_WINDOW_SIZE: LogicalSize<f32> = LogicalSize::new(1000.0f32, 600.0);

pub struct Context {
    _instance: Arc<Instance>,
    pub window: Arc<Window>, // for get inner size and request redraw
    pub viewport: Viewport,  // just for pipeline creation
    pub device: Arc<Device>,
    pub queue: Arc<Queue>, // for submitting command buffers
    pub allocators: Allocators,
    pub swapchain: Arc<Swapchain>, // swapchain recreation and image presenting
    pub images: Vec<Arc<Image>>,
}

impl Context {
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

        println!(
            "[Render Context Info]\nswapchain image count: {}\nQueue family: {}\nSwapchain format: {:?}",
            images.len(),
            queue_family_index,
            swapchain.image_format(),
        );

        // // auto focus window
        // window.focus_window();

        Self {
            _instance: instance,
            window,
            viewport,
            device,
            queue,
            allocators,
            swapchain,
            images,
        }
    }

    pub fn get_image_count(&self) -> usize {
        self.images.len()
        // self.swapchain.image_count() as usize
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
        self.images = new_images;
    }
    pub fn handle_window_resize(&mut self) {
        self.recreate_swapchain();
        self.viewport.extent = self.window.inner_size().into();
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
    pub fn flush_next_future<F>(
        &self,
        previous_future: Box<dyn GpuFuture>,
        swapchain_acquire_future: SwapchainAcquireFuture,
        image_i: u32,
        build_commands: F,
    ) -> Result<Fence, Validated<VulkanError>>
    where
        F: FnOnce(&mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>),
    {
        // create builder
        let mut builder = command_buffer::AutoCommandBufferBuilder::primary(
            &self.allocators.command_buffer,
            self.queue.queue_family_index(),
            command_buffer::CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        build_commands(&mut builder);

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

    // pub fn get_resource_loader(&self) -> ResourceLoader {
    //     ResourceLoader { context: &self }
    // }
}

// pub struct ResourceLoader<'a> {
//     context: &'a Context,
//     // draw_system: &'a mut DrawSystem<GPUObjectData, Matrix4<f32>>,
// }

// impl<'a> ResourceLoader<'a> {
//     pub fn load_texture(&self, path: &Path) -> Arc<ImageView> {
//         load_texture(&self.context.allocators, &self.context.queue, path)
//     }
//     pub fn load_sampler(&self, filter: vulkano::image::sampler::Filter) -> Arc<Sampler> {
//         create_sampler(self.context.device.clone(), filter)
//     }
//     pub fn load_mesh(
//         &self,
//         vertices: Vec<VertexFull>,
//         indices: Vec<u32>,
//     ) -> Arc<Buffers<VertexFull>> {
//         Arc::new(Buffers::initialize_device_local(
//             &self.context.allocators,
//             self.context.queue.clone(),
//             vertices,
//             indices,
//         ))
//     }

//     pub fn load_pipeline<V: Vertex>(
//         &self,
//         vs: Arc<ShaderModule>,
//         fs: Arc<ShaderModule>,
//         subpass: Subpass,
//         dynamic_bindings: impl IntoIterator<Item = (usize, u32)> + Clone,
//         pipeline_type: crate::vulkano_objects::pipeline::PipelineType,
//     ) -> PipelineHandler<V> {
//         PipelineHandler::new(
//             self.context.device.clone(),
//             vs.entry_point("main").unwrap(),
//             fs.entry_point("main").unwrap(),
//             self.context.viewport.clone(),
//             subpass,
//             dynamic_bindings,
//             pipeline_type,
//         )
//     }
//     pub fn load_pipelines<V: Vertex, const SHADERS: usize>(
//         &self,
//         modules: [(Arc<ShaderModule>, Arc<ShaderModule>); SHADERS],
//         subpass: Subpass,
//         dynamic_bindings: impl IntoIterator<Item = (usize, u32)> + Clone,
//         pipeline_type: crate::vulkano_objects::pipeline::PipelineType,
//     ) -> [PipelineHandler<V>; SHADERS] {
//         modules.map(|(vs, fs)| {
//             self.load_pipeline(
//                 vs,
//                 fs,
//                 subpass.clone(),
//                 dynamic_bindings.clone(), // [(0, 0)],
//                 pipeline_type,
//             )
//         })
//     }

//     /// creates a material of the given pipeline with a corresponding descriptor set as set 2
//     pub fn init_material(
//         &self,
//         shader: &mut Shader,
//         descriptor_writes: impl IntoIterator<Item = WriteDescriptorSet>,
//     ) -> RenderSubmit {
//         shader.add_material(Some(
//             PersistentDescriptorSet::new(
//                 &self.context.allocators.descriptor_set,
//                 shader
//                     .pipeline
//                     .layout()
//                     .set_layouts()
//                     .get(2)
//                     .unwrap()
//                     .clone(),
//                 descriptor_writes,
//                 [],
//             )
//             .unwrap(), // pipeline_group.create_material_set(&self.context.allocators, descriptor_writes),
//         ))
//     }
//     pub fn create_material_buffer<T: BufferContents>(
//         &self,
//         data: T,
//         usage: BufferUsage,
//     ) -> Subbuffer<T> {
//         buffers::create_material_buffer(&self.context.allocators, data, usage)
//     }
//     // pub fn build_material
//     // pub fn
// }
