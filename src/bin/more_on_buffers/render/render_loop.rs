use std::path::Path;
use std::sync::Arc;

use cgmath::{vec3, Matrix4, Rad, SquareMatrix};

use vulkano::{sync::GpuFuture, Validated, VulkanError};

use vulkano_template::{
    shaders::basic::fs::SceneData,
    vulkano_objects::buffers::{DynamicBuffer, Uniform},
};
use winit::event_loop::EventLoop;

use super::{
    render_data::{mesh::Mesh, render_object::RenderObject},
    renderer::{Fence, Renderer},
    CameraData, TransformData,
};
use vulkano_template::{game_objects::Square, models::SquareModel, shaders::basic};

pub struct RenderLoop {
    renderer: Renderer,
    recreate_swapchain: bool,
    window_resized: bool,
    fences: Vec<Option<Arc<Fence>>>,
    camera_descriptor: Vec<Uniform<CameraData>>,
    scenes_buffer: DynamicBuffer<SceneData>,
    previous_fence_i: u32,
    total_seconds: f32,
    render_objects: Vec<RenderObject<TransformData>>,
}

impl RenderLoop {
    pub fn new(event_loop: &EventLoop<()>) -> Self {
        let mut renderer = Renderer::initialize(event_loop);
        let frames_in_flight = renderer.get_image_count();
        let fences: Vec<Option<Arc<Fence>>> = vec![None; frames_in_flight];

        // materials
        let vertex_shader = basic::vs::load(renderer.clone_device())
            .expect("failed to create shader module")
            .entry_point("main")
            .unwrap();
        let fragment_shader = basic::fs::load(renderer.clone_device())
            .expect("failed to create shader module")
            .entry_point("main")
            .unwrap();

        let material_id = String::from("basic");
        renderer.init_material(material_id.clone(), vertex_shader, fragment_shader);

        // meshes
        let path = Path::new(
            "C:/Users/dolbp/OneDrive/Documents/GitHub/RUSTY/vulkano-template/models/gun.obj",
        );
        let (vertices, indices) = Mesh::from_obj(path).decompose();
        let gun_id = String::from("gun");
        renderer.init_mesh(gun_id.clone(), vertices, indices);

        let path = Path::new(
            "C:/Users/dolbp/OneDrive/Documents/GitHub/RUSTY/vulkano-template/models/suzanne.obj",
        );
        let (vertices, indices) = Mesh::from_obj(path).decompose();
        let suz_id = String::from("suzanne");
        renderer.init_mesh(suz_id.clone(), vertices, indices);

        let (vertices, indices) = Mesh::from_model::<SquareModel>().decompose();
        let square_id = String::from("square");
        renderer.init_mesh(square_id.clone(), vertices, indices);

        // objects
        let initial_uniform = TransformData {
            data: [0., 0., 0., 0.],
            render_matrix: (cgmath::Matrix4::identity()).into(),
        };
        let controlled_obj =
            renderer.add_render_object(suz_id, material_id.clone(), initial_uniform);
        let mut square_obj = renderer.add_render_object(square_id, material_id, initial_uniform);
        square_obj.update_transform([0., 1., 0.], cgmath::Rad(0.));

        // camera descriptors TODO: create independent layout not based on mat
        let initial_uniform = CameraData {
            view: (cgmath::Matrix4::identity()).into(),
            proj: (cgmath::Matrix4::identity()).into(),
            view_proj: (cgmath::Matrix4::identity()).into(),
        };
        let (scenes_buffer, camera_descriptor) =
            renderer.create_scene_buffers(&String::from("basic"), initial_uniform);

        Self {
            renderer,
            recreate_swapchain: false,
            window_resized: false,
            fences,
            camera_descriptor,
            scenes_buffer,
            previous_fence_i: 0,
            total_seconds: 0.0,
            render_objects: vec![controlled_obj, square_obj],
        }
    }

    fn update_render_objects(&mut self, transform_data: &Square, image_i: u32) {
        // update uniform data
        self.render_objects[0].update_transform(
            [transform_data.position[0], transform_data.position[1], 0.],
            cgmath::Rad(0.),
        );
        for obj in &self.render_objects {
            obj.update_uniform(image_i);
        }

        // update camera
        let cam_pos = vec3(0., 0., 2.);
        let translation = Matrix4::from_translation(-cam_pos);
        let rotation =
            Matrix4::from_axis_angle([0., 1., 0.].into(), cgmath::Rad(self.total_seconds * 1.));
        let view = translation * rotation;
        let mut projection = cgmath::perspective(Rad(1.2), 1., 0.1, 200.);
        projection.y.y *= -1.;

        let mut cam_uniform_contents = self.camera_descriptor[image_i as usize]
            .0
            .write()
            .unwrap_or_else(|e| panic!("Failed to write to camera uniform buffer\n{}", e));
        cam_uniform_contents.view = view.into();
        cam_uniform_contents.proj = projection.into();
        cam_uniform_contents.view_proj = (projection * view).into();

        // update scene data
        let current_scene = self.scenes_buffer.reinterpret(image_i as usize); //clone().index(image_i.into());
        let mut scene_uniform_contents = current_scene
            .write()
            .unwrap_or_else(|e| panic!("Failed to write to scene uniform buffer\n{}", e));
        scene_uniform_contents.ambient_color =
            [self.total_seconds.sin(), 0., self.total_seconds.cos(), 1.];
    }

    /// update renderer and draw upcoming image
    pub fn update(&mut self, transform_data: &Square, seconds_passed: f32) {
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
        if let Some(image_fence) = &self.fences[image_i as usize] {
            image_fence.wait(None).unwrap();
        }

        self.update_render_objects(transform_data, image_i);

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

        // RENDER
        let result = self.renderer.flush_next_future(
            previous_future,
            acquire_future,
            image_i,
            &self.render_objects,
            self.camera_descriptor[image_i as usize].1.clone(),
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
        self.previous_fence_i = image_i;
    }

    pub fn handle_window_resize(&mut self) {
        // impacts the next update
        self.window_resized = true;
    }
}
