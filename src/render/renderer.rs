use std::{collections::hash_map::HashMap, sync::Arc};

use crate::{
    shaders::basic::{
        fs::GPUSceneData,
        vs::{GPUCameraData, GPUObjectData},
    },
    vulkano_objects::{
        self,
        allocators::Allocators,
        buffers::{self, create_storage_buffers},
        pipeline::PipelineWrapper,
    },
};
use vulkano::{
    buffer::Subbuffer,
    command_buffer::{self, RenderPassBeginInfo},
    descriptor_set::{DescriptorSetWithOffsets, PersistentDescriptorSet, WriteDescriptorSet},
    device::{Device, DeviceCreateInfo, DeviceExtensions, Queue, QueueCreateInfo},
    image::{sampler::Sampler, view::ImageView, Image},
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
use winit::{
    dpi::LogicalSize,
    event_loop::EventLoop,
    window::{CursorGrabMode, Window, WindowBuilder},
};

use super::render_data::{material::Material, render_object::RenderObject};

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
    pub window: Arc<Window>, // pending refactor with swapchain
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,
    swapchain: Arc<Swapchain>,
    images: Vec<Arc<Image>>, // only used for getting image count
    render_pass: Arc<RenderPass>,
    framebuffers: Vec<Arc<Framebuffer>>, // deferred examples remakes fb's every frame
    pub allocators: Allocators,
    viewport: Viewport,
    pipelines: HashMap<String, PipelineWrapper>,
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

        let queue = queues.next().unwrap();

        let (swapchain, images) =
            vulkano_objects::swapchain::create_swapchain(&physical_device, device.clone(), surface);

        let allocators = Allocators::new(device.clone());

        let render_pass =
            vulkano_objects::render_pass::create_render_pass(device.clone(), swapchain.clone());
        let framebuffers = vulkano_objects::render_pass::create_framebuffers_from_swapchain_images(
            &images,
            render_pass.clone(),
            &allocators,
        );

        let viewport = Viewport {
            extent: window.inner_size().into(),
            ..Default::default() // offset: [0.0, 0.0],
                                 // depth_range: 0.0..=1.0,
        };

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
            swapchain,
            images,
            render_pass,
            framebuffers,
            allocators,
            viewport,
            pipelines: HashMap::new(),
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
        self.framebuffers = vulkano_objects::render_pass::create_framebuffers_from_swapchain_images(
            &self.images,
            self.render_pass.clone(),
            &self.allocators,
        );
    }

    /// recreates swapchain and framebuffers, followed by the pipeline with new viewport dimensions
    pub fn handle_window_resize(&mut self) {
        self.recreate_swapchain();
        self.viewport.extent = self.window.inner_size().into();

        for (_, v) in &mut self.pipelines {
            v.recreate_pipeline(
                self.device.clone(),
                self.render_pass.clone(),
                self.viewport.clone(),
            );
        }
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
    pub fn flush_next_future(
        &self,
        previous_future: Box<dyn GpuFuture>,
        swapchain_acquire_future: SwapchainAcquireFuture,
        image_i: u32,
        render_objects: &Vec<RenderObject>,
        global_descriptor: DescriptorSetWithOffsets,
        objects_descriptor: DescriptorSetWithOffsets,
    ) -> Result<Fence, Validated<VulkanError>> {
        let mut builder = command_buffer::AutoCommandBufferBuilder::primary(
            &self.allocators.command_buffer,
            self.queue.queue_family_index(),
            command_buffer::CommandBufferUsage::OneTimeSubmit,
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

        let mut last_pipe_id = &String::new();
        let mut last_mat = None;
        let mut last_mesh = None;
        let mut last_buffer_len = 0;
        // println!(
        //     "Data size: {}, Calculated alignment: {}",
        //     size_of::<GPUSceneData>(),
        //     align
        // );
        // println!("Begin draw");
        for (index, render_obj) in render_objects.iter().enumerate() {
            // material
            // println!(
            //     "[Rendering Obj] Mesh ID: {}, Mat ID: {}",
            //     render_obj.mesh_id, render_obj.material_id
            // );
            match last_mat {
                Some(old_mat) if Arc::ptr_eq(old_mat, &render_obj.material) => {
                    // println!("Same material, skipping...");
                }
                _ => {
                    let material = &render_obj.material;

                    // pipeline
                    if last_pipe_id != &material.pipeline_id {
                        let pipeline = &self.pipelines[&material.pipeline_id].pipeline;
                        builder
                            .bind_pipeline_graphics(pipeline.clone())
                            .unwrap()
                            .bind_descriptor_sets(
                                PipelineBindPoint::Graphics,
                                pipeline.layout().clone(),
                                0,
                                vec![global_descriptor.clone(), objects_descriptor.clone()],
                            )
                            .unwrap();

                        last_pipe_id = &material.pipeline_id;
                    }

                    material.bind_sets(&mut builder);

                    last_mat = Some(&render_obj.material);
                }
            }

            // mesh (vertices and indicies)
            match last_mesh {
                Some(old_mesh) if Arc::ptr_eq(old_mesh, &render_obj.mesh) => {
                    // println!("Same mesh, skipping...");
                }
                _ => {
                    let buffers = &render_obj.mesh;
                    let index_buffer = buffers.get_index();
                    let index_buffer_length = index_buffer.len();

                    builder
                        .bind_vertex_buffers(0, buffers.get_vertex())
                        .unwrap()
                        .bind_index_buffer(index_buffer)
                        .unwrap();

                    last_mesh = Some(&render_obj.mesh);
                    last_buffer_len = index_buffer_length;
                }
            }

            // draw
            builder
                .draw_indexed(last_buffer_len as u32, 1, 0, 0, index as u32)
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

    pub fn init_pipeline(
        &mut self,
        id: String,
        vertex_shader: EntryPoint,
        fragment_shader: EntryPoint,
    ) {
        self.pipelines.insert(
            id,
            PipelineWrapper::new(
                self.device.clone(),
                vertex_shader.clone(),
                fragment_shader.clone(),
                self.viewport.clone(),
                self.render_pass.clone(),
            ),
        );
    }
    pub fn get_pipeline(&self, id: &String) -> &PipelineWrapper {
        &self.pipelines[id]
    }

    fn init_material_with_sets(
        &mut self,
        pipeline_id: String,
        material_sets: Vec<Arc<PersistentDescriptorSet>>,
    ) -> Arc<Material> {
        let layout = self.pipelines[&pipeline_id].layout().clone();

        Arc::new(Material {
            layout,
            material_sets,
            pipeline_id,
        })
    }
    pub fn init_material(&mut self, pipeline_id: String) -> Arc<Material> {
        self.init_material_with_sets(pipeline_id, vec![])
    }
    pub fn init_material_with_texture(
        &mut self,
        pipeline_id: String,
        texture: Arc<ImageView>,
        sampler: Arc<Sampler>,
    ) -> Arc<Material> {
        let set = PersistentDescriptorSet::new(
            &self.allocators.descriptor_set,
            self.pipelines[&pipeline_id]
                .layout()
                .set_layouts()
                .get(2)
                .unwrap()
                .clone(),
            [WriteDescriptorSet::image_view_sampler(0, texture, sampler)],
            [],
        )
        .unwrap();
        self.init_material_with_sets(pipeline_id, vec![set])
    }

    /// See `buffers::create_global_descriptors`
    pub fn create_scene_buffers(
        &self,
        pipeline_id: &String,
    ) -> Vec<(
        Subbuffer<GPUCameraData>,
        Subbuffer<GPUSceneData>,
        DescriptorSetWithOffsets,
    )> {
        buffers::create_global_descriptors::<GPUCameraData, GPUSceneData>(
            &self.allocators,
            &self.device,
            self.pipelines[pipeline_id]
                .layout()
                .set_layouts()
                .get(0)
                .unwrap()
                .clone(),
            self.swapchain.image_count() as usize,
        )
    }

    pub fn create_object_buffers(
        &self,
        pipeline_id: &String,
    ) -> Vec<(Subbuffer<[GPUObjectData]>, Arc<PersistentDescriptorSet>)> {
        create_storage_buffers(
            &self.allocators,
            self.pipelines[pipeline_id]
                .layout()
                .set_layouts()
                .get(1)
                .unwrap()
                .clone(),
            self.swapchain.image_count() as usize,
            10000,
        )
    }

    pub fn debug_assets(&self) {
        println!("Pipelines: {:?}", self.pipelines.keys(),);
    }
}
