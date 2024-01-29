use std::sync::Arc;
use std::vec;

use cgmath::Matrix4;
use vulkano::{sync::GpuFuture, Validated, VulkanError};

use winit::event_loop::EventLoop;

use super::renderer::PrimeRenderer;
use super::{context::Context, context::Fence, render_data::render_object::RenderObject};

use crate::shaders::lighting::{DirectionLight, PointLight};

pub struct RenderLoop {
    pub context: Context,

    // pub render_pass: Arc<RenderPass>,
    // framebuffers: Vec<Arc<Framebuffer>>, // for starting renderpass (deferred examples remakes fb's every frame)
    // pub attachments: FramebufferAttachments, // misc attachments (depth, diffuse e.g)
    // pub draw_system: DrawSystem<GPUObjectData, Matrix4<f32>>,
    // pub lighting_system: LightingSystem,
    recreate_swapchain: bool,
    window_resized: bool,
    fences: Vec<Option<Arc<Fence>>>,
    previous_frame_i: u32,
}

impl RenderLoop {
    pub fn new(event_loop: &EventLoop<()>) -> Self {
        let context: Context = Context::initialize(event_loop);

        // let render_pass = vulkano_objects::render_pass::create_deferred_render_pass(
        //     context.device.clone(),
        //     context.swapchain.clone(),
        // );
        // let (attachments, framebuffers) =
        //     vulkano_objects::render_pass::create_deferred_framebuffers_from_images(
        //         &context.images,
        //         render_pass.clone(),
        //         &context.allocators,
        //     );

        // let draw_system = Self::init_draw_system(&context, render_pass.clone());
        // let lighting_system = LightingSystem::new(&context, &render_pass, &attachments);
        let fences = vec![None; context.get_image_count()];

        Self {
            context,

            // render_pass,
            // framebuffers,
            // attachments,
            // draw_system,
            // lighting_system,
            recreate_swapchain: false,
            window_resized: false,
            fences,
            previous_frame_i: 0,
        }
    }

    /// update renderer and draw upcoming image
    pub fn update<'a, R, L, D>(
        &mut self,
        renderer: PrimeRenderer<'a, R, L, D>, // camera_data: &Camera,
                                              // render_objects: impl Iterator<Item = &'a Arc<RenderObject<Matrix4<f32>>>>,
                                              // point_lights: impl IntoIterator<Item = PointLight>,
                                              // dir_lights: impl IntoIterator<Item = DirectionLight>,
                                              // ambient_color: impl Into<[f32; 4]>,
    ) where
        R: Iterator<Item = &'a Arc<RenderObject<Matrix4<f32>>>>,
        L: IntoIterator<Item = PointLight>,
        D: IntoIterator<Item = DirectionLight>,
    {
        // check zero sized window
        let image_extent: [u32; 2] = self.context.window.inner_size().into();
        if image_extent.contains(&0) {
            return;
        }

        // do recreation if necessary
        if self.window_resized {
            self.window_resized = false;
            self.handle_window_resize();
            renderer.renderer.recreate_framebuffers(&self.context);
            renderer.renderer.recreate_pipelines(&self.context);
        } else if self.recreate_swapchain {
            self.recreate_swapchain();
            renderer.renderer.recreate_framebuffers(&self.context);
        }

        // get upcoming image to display and future of when it is ready
        let (image_i, suboptimal, acquire_future) = match self.context.acquire_swapchain_image() {
            Ok(r) => r,
            Err(Validated::Error(VulkanError::OutOfDate)) => {
                self.recreate_swapchain = true;
                return;
            }
            Err(e) => panic!("Failed to acquire next image: {:?}", e),
        };
        if suboptimal {
            self.recreate_swapchain = true;
        }

        // wait for upcoming image to be ready (it should be by this point)
        let index = image_i as usize;
        if let Some(image_fence) = &mut self.fences[index] {
            // image_fence.wait(None).unwrap();
            image_fence.cleanup_finished();
        }

        let renderer = renderer.upload_data(index);

        // cam matrcies
        // let extends = self.context.window.inner_size();
        // let aspect = extends.width as f32 / extends.height as f32;
        // let proj = camera_data.projection_matrix(aspect);
        // let view = camera_data.view_matrix();
        // let view_proj = proj * view;
        // let inv_view_proj = view_proj.inverse_transform().unwrap();
        // let global_data = GPUGlobalData {
        //     view: view.into(),
        //     proj: proj.into(),
        //     view_proj: view_proj.into(),
        //     inv_view_proj: inv_view_proj.into(),
        // };

        // self.draw_system
        //     .upload_draw_data(index, render_objects, global_data);

        // // println!("Projection view:");
        // // let matrix: [[f32; 4]; 4] = proj_view.clone().into();
        // // for x in matrix {
        // //     println!("{:11}, {:11}, {:11}, {:11},", x[0], x[1], x[2], x[3]);
        // // }

        // self.lighting_system.upload_lights(
        //     point_lights,
        //     dir_lights,
        //     ambient_color,
        //     global_data,
        //     index,
        // );

        // logic that uses the GPU resources that are currently not used (have been waited upon)
        let something_needs_all_gpu_resources = false;
        let previous_future = match self.fences[self.previous_frame_i as usize].clone() {
            None => self.context.synchronize().boxed(),
            Some(fence) => {
                if something_needs_all_gpu_resources {
                    fence.wait(None).unwrap();
                }
                fence.boxed()
            }
        };
        if something_needs_all_gpu_resources {
            // logic that can use every GPU resource (the GPU is sleeping)
        }

        // RENDER
        // println!("[Pre-render state] seconds_passed: {}, image_i: {}, window_resized: {}, recreate_swapchain: {}", seconds_passed, image_i, self.window_resized, self.recreate_swapchain);
        let result = self.context.flush_next_future(
            previous_future,
            acquire_future,
            image_i,
            |b| renderer.build_command_buffer(index, b), // |b| {
                                                         //     Self::render(
                                                         //         self.framebuffers[index].clone(),
                                                         //         &mut self.draw_system,
                                                         //         &mut self.lighting_system,
                                                         //         index,
                                                         //         b,
                                                         //     )
                                                         // },
        );
        // replace fence of upcoming image with new one
        self.fences[image_i as usize] = match result {
            Ok(fence) => Some(Arc::new(fence)),
            Err(Validated::Error(VulkanError::OutOfDate)) => {
                self.recreate_swapchain = true;
                None
            }
            Err(e) => {
                println!("Failed to flush future: {:?}", e);
                None
            }
        };
        self.previous_frame_i = image_i;
    }

    pub fn handle_window_resize(&mut self) {
        // impacts the next update
        self.window_resized = true;
    }
    pub fn handle_window_wait(&self) {
        self.context.window.request_redraw();
    }

    fn recreate_swapchain(&mut self) {
        self.recreate_swapchain = false;
        self.context.recreate_swapchain();
        // (self.attachments, self.framebuffers) =
        //     vulkano_objects::render_pass::create_deferred_framebuffers_from_images(
        //         &self.context.images,
        //         self.render_pass.clone(),
        //         &self.context.allocators,
        //     );
        // self.lighting_system
        //     .recreate_descriptor(&self.context, &self.attachments);
    }
}

// pub trait RenderUpload<'a, O, D>
// where
//     O: Iterator<Item = &'a Arc<RenderObject<Matrix4<f32>>>>,
//     D: IntoIterator<Item = DirectionLight>,
// {
//     fn get_scene_data(&self, extends: &winit::dpi::PhysicalSize<u32>) -> GPUGlobalData;
//     fn get_render_objects(&'a self) -> O;
//     fn get_point_lights(&'a self) -> Box<dyn Iterator<Item = PointLight> + '_>;
//     fn get_direction_lights(&self) -> D;
//     fn get_ambient_color(&self) -> [f32; 4];
// }
