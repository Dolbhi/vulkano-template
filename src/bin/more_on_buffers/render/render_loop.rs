use std::path::Path;
use std::sync::Arc;

use vulkano::swapchain::AcquireError;
use vulkano::sync::{FlushError, GpuFuture};
use vulkano_template::game_objects::Square;
use vulkano_template::models::{Mesh, SquareModel};
use vulkano_template::shaders::movable_square;
use winit::event_loop::EventLoop;

use crate::render::renderer::{Fence, Renderer};

// use super::render_object::RenderObject;
use super::UniformData;

pub struct RenderLoop {
    renderer: Renderer,
    recreate_swapchain: bool,
    window_resized: bool,
    fences: Vec<Option<Arc<Fence>>>,
    previous_fence_i: u32,
    total_seconds: f32,
    controlled_i: usize,
}

impl RenderLoop {
    pub fn new(event_loop: &EventLoop<()>) -> Self {
        let mut renderer = Renderer::initialize(event_loop);
        let frames_in_flight = renderer.get_image_count();
        let fences: Vec<Option<Arc<Fence>>> = vec![None; frames_in_flight];

        // materials
        let vertex_shader = movable_square::vs::load(renderer.clone_device())
            .expect("failed to create shader module");
        let fragment_shader = movable_square::fs::load(renderer.clone_device())
            .expect("failed to create shader module");

        let material_id = String::from("basic");
        renderer.init_material(material_id.clone(), vertex_shader, fragment_shader);

        // meshes
        let path = Path::new(
            "C:/Users/dolbp/OneDrive/Documents/GitHub/RUSTY/vulkano-template/models/gun.obj",
        );
        let mesh = Mesh::from_obj(path);
        let mesh_id = String::from("gun");
        renderer.init_mesh(mesh_id.clone(), mesh);

        let square_mesh = Mesh::from_model::<SquareModel>();
        let square_id = String::from("square");
        renderer.init_mesh(square_id.clone(), square_mesh);

        // objects
        let cam_pos = cgmath::vec3(0., 0., 2.);
        let view = cgmath::Matrix4::from_translation(-cam_pos);
        let projection = cgmath::perspective(cgmath::Rad(1.2), 1., 0.1, 200.);
        let initial_uniform = UniformData {
            data: [0., 0., 0., 0.],
            render_matrix: (projection * view).into(),
        };
        let controlled_i = renderer.add_render_object(
            mesh_id,
            material_id,
            renderer.get_image_count(),
            initial_uniform,
        );

        Self {
            renderer,
            recreate_swapchain: false,
            window_resized: false,
            fences,
            previous_fence_i: 0,
            total_seconds: 0.0,
            controlled_i,
        }
    }

    /// update renderer and draw upcoming image
    pub fn update(&mut self, render_object: &Square, seconds_passed: f32) {
        // stuff
        self.total_seconds += seconds_passed;

        // do recreation if necessary
        if self.window_resized {
            self.window_resized = false;
            self.recreate_swapchain = false;
            self.renderer.handle_window_resize();
        } else if self.recreate_swapchain {
            self.recreate_swapchain = false;
            self.renderer.recreate_swapchain();
            // self.renderer.recreate_cb();
        }

        // get upcoming image to display as well as corresponding future
        let (image_i, suboptimal, acquire_future) = match self.renderer.acquire_swapchain_image() {
            Ok(r) => r,
            Err(AcquireError::OutOfDate) => {
                self.recreate_swapchain = true;
                return;
            }
            Err(e) => panic!("Failed to acquire next image: {:?}", e),
        };

        if suboptimal {
            self.recreate_swapchain = true;
        }

        // wait for upcoming image to be ready (it should be by this point)
        if let Some(image_fence) = &self.fences[image_i as usize] {
            image_fence.wait(None).unwrap();
        }

        // update uniform data
        self.renderer
            .get_render_object(self.controlled_i)
            .update_uniform(image_i, render_object, cgmath::Rad(self.total_seconds * 1.));

        // logic that uses the GPU resources that are currently not used (have been waited upon)
        let something_needs_all_gpu_resources = false;
        let previous_future = match self.fences[self.previous_fence_i as usize].clone() {
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

        let result = self
            .renderer
            .flush_next_future(previous_future, acquire_future, image_i);

        // replace fence of upcoming image with new one
        self.fences[image_i as usize] = match result {
            Ok(fence) => Some(Arc::new(fence)),
            Err(FlushError::OutOfDate) => {
                self.recreate_swapchain = true;
                None
            }
            Err(e) => {
                println!("Failed to flush future: {:?}", e);
                None
            }
        };

        self.previous_fence_i = image_i;
    }

    pub fn handle_window_resize(&mut self) {
        // impacts the next update
        self.window_resized = true;
    }
}
