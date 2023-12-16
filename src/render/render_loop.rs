use std::sync::Arc;
use std::vec;

use cgmath::Matrix4;
use vulkano::{sync::GpuFuture, Validated, VulkanError};

use winit::event_loop::EventLoop;

use super::renderer::Fence;
use super::{render_data::render_object::RenderObject, renderer::Renderer, DrawSystem};

use crate::{
    game_objects::Camera,
    shaders::draw::{self, GPUObjectData},
};

pub struct RenderLoop {
    pub renderer: Renderer,
    pub render_data: DrawSystem<GPUObjectData, Matrix4<f32>>,
    recreate_swapchain: bool,
    window_resized: bool,
    fences: Vec<Option<Arc<Fence>>>,
    previous_frame_i: u32,
}

impl RenderLoop {
    pub fn new(event_loop: &EventLoop<()>) -> Self {
        let renderer = Renderer::initialize(event_loop);

        let render_data = Self::init_render_objects(&renderer);

        let fences = vec![None; renderer.swapchain.image_count() as usize]; //(0..frames.len()).map(|_| None).collect();

        Self {
            renderer,
            render_data,
            recreate_swapchain: false,
            window_resized: false,
            fences,
            previous_frame_i: 0,
        }
    }

    /// update renderer and draw upcoming image
    pub fn update<'a>(
        &mut self,
        camera_data: &Camera,
        render_objects: impl Iterator<Item = &'a Arc<RenderObject<Matrix4<f32>>>>,
    ) {
        // check zero sized window
        let image_extent: [u32; 2] = self.renderer.window.inner_size().into();
        if image_extent.contains(&0) {
            return;
        }

        // do recreation if necessary
        if self.window_resized {
            self.window_resized = false;
            self.recreate_swapchain = false;
            self.renderer.recreate_swapchain();
            self.render_data.recreate_pipelines(&self.renderer);
        } else if self.recreate_swapchain {
            self.recreate_swapchain = false;
            self.renderer.recreate_swapchain();
        }

        // get upcoming image to display and future of when it is ready
        let (image_i, suboptimal, acquire_future) = match self.renderer.acquire_swapchain_image() {
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
        if let Some(image_fence) = &mut self.fences[image_i as usize] {
            // image_fence.wait(None).unwrap();
            image_fence.cleanup_finished();
        }

        let extends = self.renderer.window.inner_size();
        self.render_data.upload_draw_data(
            render_objects,
            camera_data,
            extends.width as f32 / extends.height as f32,
            image_i,
        );

        // logic that uses the GPU resources that are currently not used (have been waited upon)
        let something_needs_all_gpu_resources = false;
        let previous_future = match self.fences[self.previous_frame_i as usize].clone() {
            None => self.renderer.synchronize().boxed(),
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
        let result = self.renderer.flush_next_future(
            previous_future,
            acquire_future,
            image_i,
            &mut self.render_data,
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
        self.renderer.window.request_redraw();
    }

    fn init_render_objects(renderer: &Renderer) -> DrawSystem<GPUObjectData, Matrix4<f32>> {
        // pipelines
        let shaders = [
            (
                draw::load_basic_vs(renderer.device.clone())
                    .expect("failed to create basic shader module"),
                draw::load_basic_fs(renderer.device.clone())
                    .expect("failed to create basic shader module"),
            ),
            (
                draw::load_phong_vs(renderer.device.clone())
                    .expect("failed to create phong shader module"),
                draw::load_phong_fs(renderer.device.clone())
                    .expect("failed to create phong shader module"),
            ),
            (
                draw::load_basic_vs(renderer.device.clone())
                    .expect("failed to create uv shader module"),
                draw::load_uv_fs(renderer.device.clone())
                    .expect("failed to create uv shader module"),
            ),
        ];

        DrawSystem::new(
            &renderer,
            shaders.map(|(v, f)| {
                (
                    v.entry_point("main").unwrap(),
                    f.entry_point("main").unwrap(),
                )
            }),
        )
    }
}
