use std::f32::consts::PI;
use std::sync::Arc;
use std::vec;

use cgmath::{Matrix4, Transform};
use vulkano::{sync::GpuFuture, Validated, VulkanError};

use winit::event_loop::EventLoop;

use super::lighting_system::LightingSystem;
use super::renderer::Fence;
use super::{render_data::render_object::RenderObject, renderer::Renderer, DrawSystem};

use crate::shaders::lighting::fs::{DirectionLight, PointLight};
use crate::{
    game_objects::Camera,
    shaders::draw::{self, GPUObjectData},
};

pub struct RenderLoop {
    pub renderer: Renderer,
    pub draw_system: DrawSystem<GPUObjectData, Matrix4<f32>>,
    pub lighting_system: LightingSystem,
    recreate_swapchain: bool,
    window_resized: bool,
    fences: Vec<Option<Arc<Fence>>>,
    previous_frame_i: u32,
}

impl RenderLoop {
    pub fn new(event_loop: &EventLoop<()>) -> Self {
        let renderer = Renderer::initialize(event_loop);

        let draw_system = Self::init_render_objects(&renderer);
        let lighting_system = LightingSystem::new(&renderer);
        let fences = vec![None; renderer.swapchain.image_count() as usize]; //(0..frames.len()).map(|_| None).collect();

        Self {
            renderer,
            draw_system,
            lighting_system,
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
            self.lighting_system.recreate_descriptor(&self.renderer);
            self.draw_system.recreate_pipelines(&self.renderer);
            self.lighting_system.recreate_pipeline(&self.renderer);
        } else if self.recreate_swapchain {
            self.recreate_swapchain = false;
            self.renderer.recreate_swapchain();
            self.lighting_system.recreate_descriptor(&self.renderer);
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

        // cam matrcies
        let extends = self.renderer.window.inner_size();
        let aspect = extends.width as f32 / extends.height as f32;
        let proj = camera_data.projection_matrix(aspect);
        let view = camera_data.view_matrix();
        let proj_view = proj * view;

        self.draw_system
            .upload_draw_data(image_i, render_objects, proj, view, proj_view);

        let points = [
            PointLight {
                color: [1.0, 0.0, 0.0, 1.0],
                position: [0.0, 5.0, -1.0, 1.0],
            },
            PointLight {
                color: [0.0, 0.0, 1.0, 1.0],
                position: [0.0, 6.0, -1.0, 1.0], //camera_data.position.extend(1.0).into(),
            },
            PointLight {
                color: [1.0, 1.0, 1.0, 1.0],
                position: camera_data.position.extend(1.0).into(),
            },
        ];
        let angle = PI / 4.;
        let cgmath::Vector3::<f32> { x, y, z } =
            cgmath::InnerSpace::normalize(cgmath::vec3(angle.sin(), -1., angle.cos()));
        let dir = DirectionLight {
            color: [1., 1., 0., 1.],
            direction: [x, y, z, 1.],
        };
        let screen_to_world = proj_view.inverse_transform().unwrap();
        self.lighting_system.upload_lights(
            points,
            [dir],
            screen_to_world,
            [0.2, 0.2, 0.2, 1.],
            image_i as usize,
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
            &mut self.draw_system,
            &mut self.lighting_system,
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
            // (
            //     draw::load_phong_vs(renderer.device.clone())
            //         .expect("failed to create phong shader module"),
            //     draw::load_phong_fs(renderer.device.clone())
            //         .expect("failed to create phong shader module"),
            // ),
            // (
            //     draw::load_basic_vs(renderer.device.clone())
            //         .expect("failed to create uv shader module"),
            //     draw::load_uv_fs(renderer.device.clone())
            //         .expect("failed to create uv shader module"),
            // ),
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
