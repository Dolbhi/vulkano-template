use std::path::Path;
use std::sync::Arc;

use vulkano::command_buffer::{CommandBufferExecFuture, PrimaryAutoCommandBuffer};
use vulkano::device::{Device, DeviceCreateInfo, DeviceExtensions, Queue, QueueCreateInfo};
use vulkano::image::SwapchainImage;
use vulkano::instance::Instance;
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::pipeline::{GraphicsPipeline, Pipeline};
use vulkano::render_pass::{Framebuffer, RenderPass};
use vulkano::shader::ShaderModule;
use vulkano::swapchain::{
    self, AcquireError, PresentFuture, Swapchain, SwapchainAcquireFuture, SwapchainCreateInfo,
    SwapchainCreationError, SwapchainPresentInfo,
};
use vulkano::sync::future::{FenceSignalFuture, JoinFuture, NowFuture};
use vulkano::sync::{self, FlushError, GpuFuture};
use vulkano_template::game_objects::Square;
use vulkano_template::models::{Mesh, Model, SquareModel};
use vulkano_template::shaders::movable_square;
use vulkano_template::vulkano_objects;
use vulkano_template::vulkano_objects::allocators::Allocators;
use vulkano_template::vulkano_objects::buffers::Buffers;
use vulkano_win::VkSurfaceBuild;
use winit::dpi::LogicalSize;
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};

pub type Fence = FenceSignalFuture<
    PresentFuture<CommandBufferExecFuture<JoinFuture<Box<dyn GpuFuture>, SwapchainAcquireFuture>>>,
>;

pub struct Renderer {
    _instance: Arc<Instance>,
    window: Arc<Window>,
    device: Arc<Device>,
    queue: Arc<Queue>,
    swapchain: Arc<Swapchain>,
    images: Vec<Arc<SwapchainImage>>,
    render_pass: Arc<RenderPass>,
    framebuffers: Vec<Arc<Framebuffer>>,
    allocators: Allocators,
    buffers: Buffers<movable_square::vs::Data>,
    vertex_shader: Arc<ShaderModule>,
    fragment_shader: Arc<ShaderModule>,
    viewport: Viewport,
    pipelines: Vec<Arc<GraphicsPipeline>>,
    pipeline_index: usize,
    command_buffers: Vec<Arc<PrimaryAutoCommandBuffer>>,
    stuff: Stuff,
}

struct Stuff {
    view_radians: cgmath::Rad<f32>,
    bg_colour: [f32; 4],
}

impl Renderer {
    pub fn initialize(event_loop: &EventLoop<()>) -> Self {
        let instance = vulkano_objects::instance::get_instance();

        let surface = WindowBuilder::new()
            .build_vk_surface(event_loop, instance.clone())
            .unwrap();

        let window = surface
            .object()
            .unwrap()
            .clone()
            .downcast::<Window>()
            .unwrap();

        window.set_title("Movable Square");
        window.set_inner_size(LogicalSize::new(600.0f32, 600.0));

        let device_extensions = DeviceExtensions {
            khr_swapchain: true,
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

        let queue = queues.next().unwrap();

        let (swapchain, images) =
            vulkano_objects::swapchain::create_swapchain(&physical_device, device.clone(), surface);

        let render_pass =
            vulkano_objects::render_pass::create_render_pass(device.clone(), swapchain.clone());
        let framebuffers = vulkano_objects::swapchain::create_framebuffers_from_swapchain_images(
            &images,
            render_pass.clone(),
        );

        let vertex_shader =
            movable_square::vs::load(device.clone()).expect("failed to create shader module");
        let fragment_shader =
            movable_square::fs::load(device.clone()).expect("failed to create shader module");

        let viewport = Viewport {
            origin: [0.0, 0.0],
            dimensions: window.inner_size().into(),
            depth_range: 0.0..1.0,
        };

        let pipeline = vulkano_objects::pipeline::create_pipeline(
            device.clone(),
            vertex_shader.clone(),
            fragment_shader.clone(),
            render_pass.clone(),
            viewport.clone(),
        );

        let allocators = Allocators::new(device.clone());

        let path = Path::new(
            "C:/Users/dolbp/OneDrive/Documents/GitHub/RUSTY/vulkano-template/models/engine.obj",
        );
        let mesh = Mesh::from_obj(path); // Mesh::from_model::<movable_square::vs::Data, SquareModel>();
        let initial_uniform = SquareModel::get_initial_uniform_data();

        let buffers = Buffers::initialize_device_local(
            &allocators,
            pipeline.layout().set_layouts().get(0).unwrap().clone(),
            images.len(),
            queue.clone(),
            mesh,
            initial_uniform,
        );

        let stuff = Stuff {
            view_radians: cgmath::Rad(0.),
            bg_colour: [0.1, 0.1, 0.1, 0.1],
        };

        let command_buffers = vulkano_objects::command_buffers::create_simple_command_buffers_2(
            &allocators,
            queue.clone(),
            pipeline.clone(),
            &framebuffers,
            &buffers,
            stuff.bg_colour.clone(),
            stuff.view_radians.clone(),
        );

        Self {
            _instance: instance,
            window,
            device,
            queue,
            swapchain,
            images,
            render_pass,
            framebuffers,
            allocators,
            buffers,
            vertex_shader,
            fragment_shader,
            viewport,
            pipelines: vec![pipeline],
            pipeline_index: 0,
            command_buffers,
            stuff,
        }
    }

    /// recreates swapchain and framebuffers
    pub fn recreate_swapchain(&mut self) {
        let (new_swapchain, new_images) = match self.swapchain.recreate(SwapchainCreateInfo {
            image_extent: self.window.inner_size().into(),
            ..self.swapchain.create_info()
        }) {
            Ok(r) => r,
            Err(SwapchainCreationError::ImageExtentNotSupported { .. }) => return,
            Err(e) => panic!("Failed to recreate swapchain: {:?}", e),
        };

        self.swapchain = new_swapchain;
        self.framebuffers = vulkano_objects::swapchain::create_framebuffers_from_swapchain_images(
            &new_images,
            self.render_pass.clone(),
        );
    }

    /// recreates swapchain and framebuffers, followed by the pipeline and command buffers with new viewport dimensions
    pub fn handle_window_resize(&mut self) {
        self.recreate_swapchain();
        self.viewport.dimensions = self.window.inner_size().into();

        self.pipelines[self.pipeline_index] = vulkano_objects::pipeline::create_pipeline(
            self.device.clone(),
            self.vertex_shader.clone(),
            self.fragment_shader.clone(),
            self.render_pass.clone(),
            self.viewport.clone(),
        );

        // self.recreate_cb();
    }

    pub fn recreate_cb(&mut self) {
        self.command_buffers = vulkano_objects::command_buffers::create_simple_command_buffers_2(
            &self.allocators,
            self.queue.clone(),
            self.pipelines[0].clone(),
            &self.framebuffers,
            &self.buffers,
            self.stuff.bg_colour.clone(),
            self.stuff.view_radians.clone(),
        );
    }

    pub fn get_image_count(&self) -> usize {
        self.images.len()
    }

    pub fn acquire_swapchain_image(
        &self,
    ) -> Result<(u32, bool, SwapchainAcquireFuture), AcquireError> {
        swapchain::acquire_next_image(self.swapchain.clone(), None)
    }

    pub fn synchronize(&self) -> NowFuture {
        let mut now = sync::now(self.device.clone());
        now.cleanup_finished();

        now
    }

    /// Join given futures which are used to execute the commands and present the swapchain image corresponding to the given image_i
    pub fn flush_next_future(
        &self,
        previous_future: Box<dyn GpuFuture>,
        swapchain_acquire_future: SwapchainAcquireFuture,
        image_i: u32,
    ) -> Result<Fence, FlushError> {
        previous_future
            .join(swapchain_acquire_future)
            .then_execute(
                self.queue.clone(),
                self.command_buffers[image_i as usize].clone(),
            )
            .unwrap()
            .then_swapchain_present(
                self.queue.clone(),
                SwapchainPresentInfo::swapchain_image_index(self.swapchain.clone(), image_i),
            )
            .then_signal_fence_and_flush()
    }

    pub fn update_uniform(&self, index: u32, square: &Square) {
        let mut uniform_content = self.buffers.uniforms[index as usize]
            .0
            .write()
            .unwrap_or_else(|e| panic!("Failed to write to uniform buffer\n{}", e));

        // uniform_content.color = square.color.into();
        uniform_content.position = square.position;
    }

    pub fn update_stuff(&mut self, colour: [f32; 4], radians: cgmath::Rad<f32>) {
        self.stuff.bg_colour = colour;
        self.stuff.view_radians = radians;
    }

    // pub fn swap_pipeline(&mut self) {
    //     self.pipeline_index = (self.pipeline_index + 1) % 2;

    //     self.command_buffers = vulkano_objects::command_buffers::create_simple_command_buffers(
    //         &self.allocators,
    //         self.queue.clone(),
    //         self.pipelines[self.pipeline_index].clone(),
    //         &self.framebuffers,
    //         &self.buffers,
    //     );
    // }
}
