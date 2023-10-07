use std::collections::hash_map::HashMap;
use std::sync::Arc;

// use cgmath::Matrix4;

// use vulkano::buffer::BufferContents;
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferExecFuture, CommandBufferUsage, RenderPassBeginInfo,
    SubpassContents,
};
use vulkano::device::{Device, DeviceCreateInfo, DeviceExtensions, Queue, QueueCreateInfo};
use vulkano::image::SwapchainImage;
use vulkano::instance::Instance;
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::pipeline::{GraphicsPipeline, Pipeline, PipelineBindPoint};
use vulkano::render_pass::{Framebuffer, RenderPass};
use vulkano::shader::ShaderModule;
use vulkano::swapchain::{
    self, AcquireError, PresentFuture, Swapchain, SwapchainAcquireFuture, SwapchainCreateInfo,
    SwapchainCreationError, SwapchainPresentInfo,
};
use vulkano::sync::future::{FenceSignalFuture, JoinFuture, NowFuture};
use vulkano::sync::{self, FlushError, GpuFuture};
// use vulkano_template::game_objects::Square;
use vulkano_template::models::Mesh;
use vulkano_template::shaders::movable_square;
use vulkano_template::vulkano_objects;
use vulkano_template::vulkano_objects::allocators::Allocators;
use vulkano_template::vulkano_objects::buffers::{create_cpu_accessible_uniforms, Buffers};
use vulkano_win::VkSurfaceBuild;
use winit::dpi::LogicalSize;
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};

use super::render_object::RenderObject;
use super::UniformData;

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
    viewport: Viewport,
    // command_buffers: Vec<Arc<PrimaryAutoCommandBuffer>>,
    mesh_buffers: HashMap<String, Buffers>,
    material_pipelines: HashMap<String, Arc<GraphicsPipeline>>,
    render_objects: Vec<RenderObject<UniformData>>,
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

        let allocators = Allocators::new(device.clone());

        let render_pass =
            vulkano_objects::render_pass::create_render_pass(device.clone(), swapchain.clone());
        let framebuffers = vulkano_objects::swapchain::create_framebuffers_from_swapchain_images(
            &images,
            render_pass.clone(),
            &allocators,
        );

        // HELL
        // let vertex_shader =
        //     movable_square::vs::load(device.clone()).expect("failed to create shader module");
        // let fragment_shader =
        //     movable_square::fs::load(device.clone()).expect("failed to create shader module");

        let viewport = Viewport {
            origin: [0.0, 0.0],
            dimensions: window.inner_size().into(),
            depth_range: 0.0..1.0,
        };

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
            viewport,
            // command_buffers,
            mesh_buffers: HashMap::new(),
            material_pipelines: HashMap::new(),
            render_objects: vec![],
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
        self.images = new_images;
        self.framebuffers = vulkano_objects::swapchain::create_framebuffers_from_swapchain_images(
            &self.images,
            self.render_pass.clone(),
            &self.allocators,
        );
    }

    /// recreates swapchain and framebuffers, followed by the pipeline with new viewport dimensions
    pub fn handle_window_resize(&mut self) {
        self.recreate_swapchain();
        self.viewport.dimensions = self.window.inner_size().into();

        let vertex_shader =
            movable_square::vs::load(self.device.clone()).expect("failed to create shader module");
        let fragment_shader =
            movable_square::fs::load(self.device.clone()).expect("failed to create shader module");

        self.material_pipelines.insert(
            self.render_objects[0].pipeline_id.clone(),
            vulkano_objects::pipeline::create_pipeline(
                self.device.clone(),
                vertex_shader.clone(),
                fragment_shader.clone(),
                self.render_pass.clone(),
                self.viewport.clone(),
            ),
        );

        // self.recreate_cb();
    }

    pub fn get_image_count(&self) -> usize {
        self.images.len()
    }

    pub fn clone_device(&self) -> Arc<Device> {
        self.device.clone()
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
        // render_objects: &Vec<RenderObject<U>>,
    ) -> Result<Fence, FlushError> {
        let mut builder = AutoCommandBufferBuilder::primary(
            &self.allocators.command_buffer,
            self.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        builder
            .begin_render_pass(
                RenderPassBeginInfo {
                    clear_values: vec![Some([0.1, 0.1, 0.1, 1.0].into()), Some(1.0.into())],
                    ..RenderPassBeginInfo::framebuffer(self.framebuffers[image_i as usize].clone())
                },
                SubpassContents::Inline,
            )
            .unwrap();
        for render_obj in &self.render_objects {
            let pipeline = &self.material_pipelines[&render_obj.pipeline_id];

            let buffers = &self.mesh_buffers[&render_obj.mesh_id];
            let index_buffer = buffers.get_index();
            let index_buffer_length = index_buffer.len();

            builder
                .bind_pipeline_graphics(pipeline.clone())
                .bind_descriptor_sets(
                    PipelineBindPoint::Graphics,
                    pipeline.layout().clone(),
                    0,
                    render_obj.get_uniforms()[image_i as usize].1.clone(),
                )
                .bind_vertex_buffers(0, buffers.get_vertex())
                .bind_index_buffer(index_buffer)
                .draw_indexed(index_buffer_length as u32, 1, 0, 0, 0)
                .unwrap();
        }
        builder.end_render_pass().unwrap();

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

    pub fn init_mesh(&mut self, id: String, mesh: Mesh) {
        let buffer = Buffers::initialize_device_local(&self.allocators, self.queue.clone(), mesh);
        self.mesh_buffers.insert(id, buffer);
    }

    pub fn init_material(
        &mut self,
        id: String,
        vertex_shader: Arc<ShaderModule>,
        fragment_shader: Arc<ShaderModule>,
    ) {
        let pipeline = vulkano_objects::pipeline::create_pipeline(
            self.device.clone(),
            vertex_shader.clone(),
            fragment_shader.clone(),
            self.render_pass.clone(),
            self.viewport.clone(),
        );
        self.material_pipelines.insert(id, pipeline);
    }

    pub fn add_render_object(
        &mut self,
        mesh_id: String,
        material_id: String,
        uniform_buffer_count: usize,
        initial_uniform: UniformData,
    ) -> usize {
        let descriptor_set_layout = self.material_pipelines[&material_id]
            .layout()
            .set_layouts()
            .get(0)
            .unwrap()
            .clone();

        let uniforms = create_cpu_accessible_uniforms(
            &self.allocators,
            descriptor_set_layout,
            uniform_buffer_count,
            initial_uniform,
        );

        let render_obj = RenderObject::new(mesh_id, material_id, uniforms);

        self.render_objects.push(render_obj);

        self.render_objects.len() - 1
    }

    // pub fn update_ro_transform(
    //     &mut self,
    //     ro_i: usize,
    //     position: [f32; 2],
    //     radians: cgmath::Rad<f32>,
    // ) {
    //     self.render_objects[ro_i].update_transform(position, radians);
    // }

    pub fn get_render_object(&mut self, ro_i: usize) -> &mut RenderObject<UniformData> {
        &mut self.render_objects[ro_i]
    }
}
