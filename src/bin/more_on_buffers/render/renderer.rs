use std::sync::Arc;
use std::{collections::hash_map::HashMap, mem::size_of};

use vulkano::{
    command_buffer::{
        AutoCommandBufferBuilder, CommandBufferExecFuture, CommandBufferUsage, RenderPassBeginInfo,
    },
    descriptor_set::{DescriptorSet, PersistentDescriptorSet},
    device::{Device, DeviceCreateInfo, DeviceExtensions, Queue, QueueCreateInfo},
    image::Image,
    instance::Instance,
    pipeline::{graphics::viewport::Viewport, Pipeline, PipelineBindPoint},
    render_pass::{Framebuffer, RenderPass},
    shader::EntryPoint,
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
use vulkano_template::vulkano_objects::buffers::DynamicBuffer;
use vulkano_template::{
    shaders::basic::fs::SceneData,
    vulkano_objects::buffers::{create_cpu_accessible_uniforms, Buffers},
    vulkano_objects::{self, buffers},
    vulkano_objects::{allocators::Allocators, buffers::Uniform},
    VertexFull,
};
use winit::dpi::LogicalSize;
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};

use super::{
    render_data::{material::Material, render_object::RenderObject},
    CameraData, TransformData,
};

pub type Fence = FenceSignalFuture<
    PresentFuture<CommandBufferExecFuture<JoinFuture<Box<dyn GpuFuture>, SwapchainAcquireFuture>>>,
>;

pub struct Renderer {
    _instance: Arc<Instance>,
    window: Arc<Window>,
    device: Arc<Device>,
    queue: Arc<Queue>,
    swapchain: Arc<Swapchain>,
    images: Vec<Arc<Image>>,
    render_pass: Arc<RenderPass>,
    framebuffers: Vec<Arc<Framebuffer>>,
    allocators: Allocators,
    viewport: Viewport,
    mesh_buffers: HashMap<String, Buffers<VertexFull>>,
    material_pipelines: HashMap<String, Material>,
}

impl Renderer {
    pub fn initialize(event_loop: &EventLoop<()>) -> Self {
        let instance = vulkano_objects::instance::get_instance(event_loop);

        let window = Arc::new(WindowBuilder::new().build(&event_loop).unwrap());
        let surface = Surface::from_window(instance.clone(), window.clone()).unwrap();

        window.set_title("Rusty Renderer");
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

        let viewport = Viewport {
            offset: [0.0, 0.0],
            extent: window.inner_size().into(),
            depth_range: 0.0..=1.0,
        };

        println!("[Renderer info] swapchain image count: {}", images.len());

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
            mesh_buffers: HashMap::new(),
            material_pipelines: HashMap::new(),
        }
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
        self.framebuffers = vulkano_objects::swapchain::create_framebuffers_from_swapchain_images(
            &self.images,
            self.render_pass.clone(),
            &self.allocators,
        );
    }

    /// recreates swapchain and framebuffers, followed by the pipeline with new viewport dimensions
    pub fn handle_window_resize(&mut self) {
        self.recreate_swapchain();
        self.viewport.extent = self.window.inner_size().into();

        for (_, v) in &mut self.material_pipelines {
            v.recreate_pipeline(
                self.device.clone(),
                self.render_pass.clone(),
                self.viewport.clone(),
            );
        }
    }

    pub fn get_image_count(&self) -> usize {
        self.images.len()
    }

    pub fn clone_device(&self) -> Arc<Device> {
        self.device.clone()
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

    fn pad_buffer_size(&self, size: usize) -> usize {
        let min_dynamic_align = self
            .device
            .physical_device()
            .properties()
            .min_uniform_buffer_offset_alignment
            .as_devicesize() as usize;

        // Round size up to the next multiple of align.
        // size_of::<B>()
        (size + min_dynamic_align - 1) & !(min_dynamic_align - 1)
    }

    /// Join given futures then execute new commands and present the swapchain image corresponding to the given image_i
    pub fn flush_next_future(
        &self,
        previous_future: Box<dyn GpuFuture>,
        swapchain_acquire_future: SwapchainAcquireFuture,
        image_i: u32,
        render_objects: &Vec<RenderObject<TransformData>>,
        global_descriptor: Arc<PersistentDescriptorSet>,
    ) -> Result<Fence, Validated<VulkanError>> {
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
                Default::default(),
            )
            .unwrap();

        let mut last_mat = &String::new();
        let mut last_mesh = &String::new();
        let mut last_buffer_len = 0;
        let align = self.pad_buffer_size(size_of::<SceneData>()) as u32;
        // println!(
        //     "Data size: {}, Calculated alignment: {}",
        //     size_of::<SceneData>(),
        //     align
        // );
        for render_obj in render_objects {
            // material (pipeline)
            let pipeline = self.material_pipelines[&render_obj.material_id].get_pipeline();
            if last_mat != &render_obj.material_id {
                builder
                    .bind_pipeline_graphics(pipeline.clone())
                    .unwrap()
                    .bind_descriptor_sets(
                        PipelineBindPoint::Graphics,
                        pipeline.layout().clone(),
                        0,
                        global_descriptor.clone().offsets([image_i * align]),
                    )
                    .unwrap();

                last_mat = &render_obj.material_id;
            }

            // mesh (vertices and indicies)
            if last_mesh != &render_obj.mesh_id {
                let buffers = &self.mesh_buffers[&render_obj.mesh_id];
                let index_buffer = buffers.get_index();
                let index_buffer_length = index_buffer.len();

                builder
                    .bind_vertex_buffers(0, buffers.get_vertex())
                    .unwrap()
                    .bind_index_buffer(index_buffer)
                    .unwrap();

                last_mesh = &render_obj.mesh_id;
                last_buffer_len = index_buffer_length;
            }

            // descriptor sets + draw
            builder
                .bind_descriptor_sets(
                    PipelineBindPoint::Graphics,
                    pipeline.layout().clone(),
                    1,
                    render_obj.clone_descriptor(image_i as usize),
                )
                .unwrap()
                .draw_indexed(last_buffer_len as u32, 1, 0, 0, 0)
                .unwrap();
        }
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

    pub fn init_mesh(&mut self, id: String, vertices: Vec<VertexFull>, indices: Vec<u32>) {
        // let (vertices, indices) = mesh.decompose();

        let buffer = Buffers::initialize_device_local(
            &self.allocators,
            self.queue.clone(),
            vertices,
            indices,
        );
        self.mesh_buffers.insert(id, buffer);
    }

    pub fn init_material(
        &mut self,
        id: String,
        vertex_shader: EntryPoint,
        fragment_shader: EntryPoint,
    ) {
        let pipeline = vulkano_objects::pipeline::window_size_dependent_pipeline(
            self.device.clone(),
            vertex_shader.clone(),
            fragment_shader.clone(),
            self.viewport.clone(),
            self.render_pass.clone(),
        );
        let mat = Material::new(vertex_shader, fragment_shader, pipeline);
        self.material_pipelines.insert(id, mat);
    }

    pub fn add_render_object(
        &self,
        mesh_id: String,
        material_id: String,
        initial_uniform: TransformData,
    ) -> RenderObject<TransformData> {
        let uniforms = create_cpu_accessible_uniforms(
            &self.allocators,
            self.material_pipelines[&material_id]
                .get_pipeline()
                .layout()
                .set_layouts()
                .get(1)
                .unwrap()
                .clone(),
            self.get_image_count(),
            initial_uniform,
        );

        // let render_obj =
        RenderObject::new(mesh_id, material_id, uniforms)

        // self.render_objects.push(render_obj);

        // self.render_objects.len() - 1
    }

    pub fn create_scene_buffers(
        &self,
        material_id: &String,
        cam_data: CameraData,
        // scene_data: SceneData,
    ) -> (DynamicBuffer<SceneData>, Vec<Uniform<CameraData>>) {
        let image_count = self.get_image_count();

        buffers::create_global_descriptors::<SceneData, CameraData>(
            &self.allocators,
            self.material_pipelines[material_id]
                .get_pipeline()
                .layout()
                .set_layouts()
                .get(0)
                .unwrap()
                .clone(),
            image_count,
            cam_data,
            self.pad_buffer_size(size_of::<SceneData>()) as usize,
        )
    }
}
