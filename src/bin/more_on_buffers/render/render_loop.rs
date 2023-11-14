// use std::mem::size_of;
use std::path::Path;
use std::sync::Arc;

use cgmath::{vec3, Matrix4, Rad};

use vulkano::descriptor_set::{DescriptorSet, PersistentDescriptorSet};
use vulkano::{sync::GpuFuture, Validated, VulkanError};

use vulkano_template::{
    shaders::basic::{fs::GPUSceneData, vs::GPUCameraData},
    vulkano_objects::buffers::DynamicBuffer,
};
use winit::event_loop::EventLoop;

use super::{
    render_data::{frame_data::FrameData, mesh::Mesh, render_object::RenderObject},
    renderer::Renderer,
};
use vulkano_template::{game_objects::Square, models::SquareModel, shaders::basic};

pub struct RenderLoop {
    renderer: Renderer,
    recreate_swapchain: bool,
    window_resized: bool,
    frames: Vec<FrameData>,
    previous_frame_i: u32,
    camera_dynamic: DynamicBuffer<GPUCameraData>,
    scene_dynamic: DynamicBuffer<GPUSceneData>,
    global_descriptor: Arc<PersistentDescriptorSet>,
    total_seconds: f32,
    render_objects: Vec<RenderObject>,
}

impl RenderLoop {
    pub fn new(event_loop: &EventLoop<()>) -> Self {
        let mut renderer = Renderer::initialize(event_loop);

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
        //      gun
        let path = Path::new(
            "C:/Users/dolbp/OneDrive/Documents/GitHub/RUSTY/vulkano-template/models/gun.obj",
        );
        let (vertices, indices) = Mesh::from_obj(path).decompose();
        let gun_id = String::from("gun");

        renderer.init_mesh(gun_id.clone(), vertices, indices);

        //      suzanne
        let path = Path::new(
            "C:/Users/dolbp/OneDrive/Documents/GitHub/RUSTY/vulkano-template/models/suzanne.obj",
        );
        let (vertices, indices) = Mesh::from_obj(path).decompose();
        let suz_id = String::from("suzanne");

        renderer.init_mesh(suz_id.clone(), vertices, indices);

        //      square
        let (vertices, indices) = Mesh::from_model::<SquareModel>().decompose();
        let square_id = String::from("square");

        renderer.init_mesh(square_id.clone(), vertices, indices);

        // objects
        let mut render_objects = Vec::<RenderObject>::with_capacity(9);
        let controlled_obj = RenderObject::new(suz_id, material_id.clone());
        render_objects.push(controlled_obj);
        for (x, y) in (-1..2)
            .flat_map(|x| (-1..2).map(move |y| (x.clone(), y)))
            .filter(|a| *a != (0, 0))
        {
            let mut square_obj = RenderObject::new(square_id.clone(), material_id.clone());
            square_obj.update_transform([x as f32, y as f32, 0.], cgmath::Rad(0.));
            render_objects.push(square_obj)
        }
        println!("Total render objs: {}", render_objects.len());

        // global descriptors TODO: 1. Group dyanamics into its own struct 2. create independent layout not based on mat
        let (camera_dynamic, scene_dynamic, global_descriptor) =
            renderer.create_scene_buffers(&String::from("basic"));

        let object_uniforms = renderer.create_object_buffers(&String::from("basic"));

        // create frame data
        let frames = object_uniforms
            .into_iter()
            .map(|(storage_buffer, object_descriptor)| {
                FrameData::new(storage_buffer, object_descriptor)
            })
            .collect();

        Self {
            renderer,
            recreate_swapchain: false,
            window_resized: false,
            frames,
            previous_frame_i: 0,
            camera_dynamic,
            scene_dynamic,
            global_descriptor,
            total_seconds: 0.0,
            render_objects,
        }
    }

    fn update_gpu_data(&mut self, transform_data: &Square, image_i: u32) {
        let image_i = image_i as usize;

        // update object data
        self.render_objects[0].update_transform(
            [transform_data.position[0], transform_data.position[1], 0.],
            cgmath::Rad(0.),
        );
        self.frames[image_i].update_objects_data(&self.render_objects);

        // update camera
        let cam_pos = vec3(0., 0., 2.);
        let translation = Matrix4::from_translation(-cam_pos);
        let rotation =
            Matrix4::from_axis_angle([0., 1., 0.].into(), cgmath::Rad(self.total_seconds * 1.));
        let view = translation * rotation;
        let mut projection = cgmath::perspective(Rad(1.2), 1., 0.1, 200.);
        projection.y.y *= -1.;

        let current_camera = self.camera_dynamic.reinterpret(image_i);
        let mut camera_uniform_contents = current_camera
            .write()
            .unwrap_or_else(|e| panic!("Failed to write to scene uniform buffer\n{}", e));
        camera_uniform_contents.view = view.into();
        // camera_uniform_contents.proj = projection.into();
        camera_uniform_contents.view_proj = (projection * view).into();

        // update scene data
        let current_scene = self.scene_dynamic.reinterpret(image_i);
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
        if let Some(image_fence) = &self.frames[image_i as usize].fence {
            image_fence.wait(None).unwrap();
        }

        self.update_gpu_data(transform_data, image_i);

        // logic that uses the GPU resources that are currently not used (have been waited upon)
        let something_needs_all_gpu_resources = false;
        let previous_future = match self.frames[self.previous_frame_i as usize].fence.clone() {
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
        // println!(
        //     "[Camera dynamics] data size: {}, alignment: {}, buffer size: {}, range end: {}, offset: {}",
        //     size_of::<GPUCameraData>(),
        //     self.camera_dynamic.align(),
        //     self.camera_dynamic.clone_buffer().size(),
        //     (0..size_of::<GPUCameraData>()).end,
        //     self.camera_dynamic.align() as u32 * image_i,
        // );
        let result = self.renderer.flush_next_future(
            previous_future,
            acquire_future,
            image_i,
            &self.render_objects,
            self.global_descriptor.clone().offsets([
                self.camera_dynamic.align() as u32 * image_i,
                self.scene_dynamic.align() as u32 * image_i,
            ]),
            self.frames[image_i as usize]
                .get_objects_descriptor()
                .clone(),
        );
        // replace fence of upcoming image with new one
        self.frames[image_i as usize].fence = match result {
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
}
