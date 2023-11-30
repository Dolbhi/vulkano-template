use std::vec;
use std::{collections::HashMap, iter::zip};
// use std::mem::size_of;
use std::path::Path;
use std::sync::Arc;

use cgmath::{vec3, InnerSpace, Vector3};
use vulkano::{sync::GpuFuture, Validated, VulkanError};

use winit::event_loop::EventLoop;

use super::renderer::Fence;
use super::{
    render_data::{
        frame_data::FrameData,
        mesh::Mesh,
        render_object::{PipelineGroup, RenderObject},
        texture::{create_sampler, load_texture},
    },
    renderer::Renderer,
};

use crate::shaders::phong;
use crate::{
    game_objects::Camera,
    shaders::{basic, uv},
    vulkano_objects::buffers::Buffers,
    VertexFull,
};

pub struct RenderLoop {
    renderer: Renderer,
    recreate_swapchain: bool,
    window_resized: bool,
    frames: Vec<FrameData>,
    fences: Vec<Option<Arc<Fence>>>,
    previous_frame_i: u32,
    total_seconds: f32,
    render_pipelines: Vec<PipelineGroup>,
    sorted_objects: HashMap<String, Vec<Arc<RenderObject>>>,
    render_objects: Vec<Arc<RenderObject>>,
}

impl RenderLoop {
    pub fn new(event_loop: &EventLoop<()>) -> Self {
        let mut renderer = Renderer::initialize(event_loop);

        let (render_pipelines, sorted_objects, render_objects) =
            Self::init_render_objects(&mut renderer);

        // global descriptors TODO: 1. Group dyanamics into its own struct 2. create independent layout not based on mat
        let layout = render_pipelines[0].pipeline.layout();
        let global_data = renderer.create_scene_buffers(layout);
        let object_uniforms = renderer.create_object_buffers(layout);

        // create frame data
        let frames: Vec<FrameData> = zip(global_data, object_uniforms)
            .into_iter()
            .map(
                |((cam_buffer, scene_buffer, global_set), (storage_buffer, object_descriptor))| {
                    let mut frame = FrameData::new(
                        cam_buffer,
                        scene_buffer,
                        global_set,
                        storage_buffer,
                        object_descriptor.into(),
                    );
                    frame.update_scene_data(
                        Some([0.2, 0.2, 0.2, 1.]),
                        None,
                        Some([0.9, 0.9, 0.6, 1.]),
                    );
                    frame
                },
            )
            .collect();

        let fences = vec![None; frames.len()]; //(0..frames.len()).map(|_| None).collect();

        Self {
            renderer,
            recreate_swapchain: false,
            window_resized: false,
            frames,
            fences,
            previous_frame_i: 0,
            total_seconds: 0.0,
            render_pipelines,
            sorted_objects,
            render_objects,
        }
    }

    /// update renderer and draw upcoming image
    pub fn update(&mut self, transform_data: &Camera, seconds_passed: f32) {
        // stuff
        self.total_seconds += seconds_passed;

        // check zero sized window
        let image_extent: [u32; 2] = self.renderer.window.inner_size().into();
        if image_extent.contains(&0) {
            return;
        }

        // do recreation if necessary
        if self.window_resized {
            self.window_resized = false;
            self.recreate_swapchain = false;
            self.renderer
                .handle_window_resize(&mut self.render_pipelines);
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
        if let Some(image_fence) = &mut self.fences[image_i as usize] {
            // image_fence.wait(None).unwrap();
            image_fence.cleanup_finished();
        }

        self.update_gpu_data(transform_data, image_i);

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
        // println!(
        //     "[Camera dynamics] data size: {}, alignment: {}, buffer size: {}, range end: {}, offset: {}",
        //     size_of::<GPUCameraData>(),
        //     self.camera_dynamic.align(),
        //     self.camera_dynamic.clone_buffer().size(),
        //     (0..size_of::<GPUCameraData>()).end,
        //     self.camera_dynamic.align() as u32 * image_i,
        // );
        let FrameData {
            global_descriptor,
            objects_descriptor,
            ..
        } = &self.frames[image_i as usize];
        let result = self.renderer.flush_next_future(
            previous_future,
            acquire_future,
            image_i,
            &self.render_pipelines,
            &mut self.sorted_objects,
            &global_descriptor,
            &objects_descriptor,
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

    fn init_render_objects(
        renderer: &mut Renderer,
    ) -> (
        Vec<PipelineGroup>,
        HashMap<String, Vec<Arc<RenderObject>>>,
        Vec<Arc<RenderObject>>,
    ) {
        // pipelines
        // let mut basic_pipeline = {
        //     let vertex_shader = basic::vs::load(renderer.device.clone())
        //         .expect("failed to create shader module")
        //         .entry_point("main")
        //         .unwrap();
        //     let fragment_shader = basic::fs::load(renderer.device.clone())
        //         .expect("failed to create shader module")
        //         .entry_point("main")
        //         .unwrap();
        //     PipelineGroup::new(renderer.create_pipeline(vertex_shader, fragment_shader))
        // };
        let mut phong_pipeline = {
            let vertex_shader = phong::vs::load(renderer.device.clone())
                .expect("failed to create shader module")
                .entry_point("main")
                .unwrap();
            let fragment_shader = phong::fs::load(renderer.device.clone())
                .expect("failed to create shader module")
                .entry_point("main")
                .unwrap();
            PipelineGroup::new(renderer.create_pipeline(vertex_shader, fragment_shader))
        };
        let mut uv_pipeline = {
            let vertex_shader = uv::vs::load(renderer.device.clone())
                .expect("failed to create shader module")
                .entry_point("main")
                .unwrap();
            let fragment_shader = uv::fs::load(renderer.device.clone())
                .expect("failed to create shader module")
                .entry_point("main")
                .unwrap();
            PipelineGroup::new(renderer.create_pipeline(vertex_shader, fragment_shader))
        };

        // Texture
        let le_texture = load_texture(
            &renderer.allocators,
            &renderer.queue,
            Path::new("models/lost_empire-RGBA.png"),
        );

        let ina_textures = [
            "models/ina/Hair_Base_Color.png",
            "models/ina/Cloth_Base_Color.png",
            "models/ina/Body_Base_Color.png",
            "models/ina/Head_Base_Color.png",
        ]
        .map(|p| load_texture(&renderer.allocators, &renderer.queue, Path::new(p)));

        let linear_sampler = create_sampler(
            renderer.device.clone(),
            vulkano::image::sampler::Filter::Linear,
        );

        // materials
        let mut objects_hash = HashMap::new();
        //  lost empire
        let le_mat_id = "lost_empire".to_string();
        phong_pipeline.add_material(
            le_mat_id.clone(),
            Some(phong_pipeline.create_material_set(
                &renderer.allocators,
                2,
                le_texture,
                linear_sampler.clone(),
            )),
        );
        objects_hash.insert(le_mat_id.clone(), vec![]);

        //  ina
        let ina_ids = ["hair", "cloth", "body", "head"].map(|s| s.to_string());
        for (id, tex) in zip(ina_ids.clone(), ina_textures) {
            phong_pipeline.add_material(
                id.clone(),
                Some(phong_pipeline.create_material_set(
                    &renderer.allocators,
                    2,
                    tex,
                    linear_sampler.clone(),
                )),
            );
            objects_hash.insert(id, vec![]);
        }

        //  uv
        let uv_mat_id = "uv".to_string();
        uv_pipeline.add_material(uv_mat_id.clone(), None);
        objects_hash.insert(uv_mat_id.clone(), vec![]);

        // meshes
        //      suzanne
        let Mesh(vertices, indices) = Mesh::from_obj(Path::new("models/suzanne.obj"))
            .pop()
            .unwrap();
        let suzanne = Arc::new(Buffers::initialize_device_local(
            &renderer.allocators,
            renderer.queue.clone(),
            vertices,
            indices,
        ));

        //      square
        let vertices = vec![
            VertexFull {
                position: [-0.25, -0.25, 0.0],
                normal: [0.0, 0.0, 1.0],
                colour: [0.0, 1.0, 0.0],
                uv: [0.0, 0.0],
            },
            VertexFull {
                position: [0.25, -0.25, 0.0],
                normal: [0.0, 0.0, 1.0],
                colour: [0.0, 1.0, 0.0],
                uv: [1.0, 0.0],
            },
            VertexFull {
                position: [-0.25, 0.25, 0.0],
                normal: [0.0, 0.0, 1.0],
                colour: [0.0, 1.0, 0.0],
                uv: [0.0, 1.0],
            },
            VertexFull {
                position: [0.25, 0.25, 0.0],
                normal: [0.0, 0.0, 1.0],
                colour: [0.0, 1.0, 0.0],
                uv: [1.0, 1.0],
            },
        ];
        let indices = vec![0, 1, 2, 2, 1, 3];
        let square = Arc::new(Buffers::initialize_device_local(
            &renderer.allocators,
            renderer.queue.clone(),
            vertices,
            indices,
        ));

        //      lost empire
        let le_meshes: Vec<Arc<Buffers<VertexFull>>> =
            Mesh::from_obj(Path::new("models/lost_empire.obj"))
                .into_iter()
                .map(|Mesh(vertices, indices)| {
                    Arc::new(Buffers::initialize_device_local(
                        &renderer.allocators,
                        renderer.queue.clone(),
                        vertices,
                        indices,
                    ))
                })
                .collect();

        //      ina
        let ina_meshes: Vec<Arc<Buffers<VertexFull>>> =
            Mesh::from_obj(Path::new("models/ina/ReadyToRigINA.obj"))
                .into_iter()
                .skip(2)
                .map(|Mesh(vertices, indices)| {
                    Arc::new(Buffers::initialize_device_local(
                        &renderer.allocators,
                        renderer.queue.clone(),
                        vertices,
                        indices,
                    ))
                })
                .collect();

        println!("[Rendering Data]");
        println!("Lost empire mesh count: {}", le_meshes.len());
        println!("Ina mesh count: {}", ina_meshes.len());

        // objects
        let mut render_objects = Vec::new();
        //  Suzanne
        render_objects.push(Arc::new(RenderObject::new(suzanne, uv_mat_id.clone())));

        //  Squares
        for (x, y, z) in [(1, 0, 0), (0, 1, 0), (0, 0, 1)] {
            let mut square_obj = RenderObject::new(square.clone(), uv_mat_id.clone());
            square_obj.update_transform([x as f32, y as f32, z as f32], cgmath::Rad(0.));

            render_objects.push(Arc::new(square_obj));
        }

        //  Ina
        for (mesh, mat_id) in zip(ina_meshes, ina_ids.clone()) {
            let mut obj = RenderObject::new(mesh, mat_id);
            obj.update_transform([0.0, 5.0, -1.0], cgmath::Rad(0.));

            render_objects.push(Arc::new(obj));
        }

        //  lost empires
        for mesh in le_meshes {
            render_objects.push(Arc::new(RenderObject::new(mesh, le_mat_id.clone())));
        }

        (
            // vec![basic_pipeline, phong_pipeline, uv_pipeline],
            vec![phong_pipeline, uv_pipeline],
            objects_hash,
            render_objects,
        )
    }

    /// write gpu data to respective buffers
    fn update_gpu_data(&mut self, camera_data: &Camera, image_i: u32) {
        let frame = &mut self.frames[image_i as usize];

        // update object data
        match Arc::get_mut(&mut self.render_objects[0]) {
            Some(obj) => {
                obj.update_transform([0., 0., 0.], cgmath::Rad(self.total_seconds));
            }
            None => {
                panic!("Unable to update render object");
            }
        }
        // sort renderobjects
        for obj in self.render_objects.iter() {
            self.sorted_objects
                .get_mut(&obj.material_id)
                .unwrap()
                .push(obj.clone());
        }
        let obj_iter = self.render_pipelines.iter().flat_map(|pipeline| {
            pipeline
                .materials
                .iter()
                .flat_map(|mat| self.sorted_objects[&mat.id].iter())
        });
        frame.update_objects_data(obj_iter);

        // update camera
        let extends = self.renderer.window.inner_size();
        frame.update_camera_data(
            camera_data.view_matrix(),
            camera_data.projection_matrix(extends.width as f32 / extends.height as f32),
        );

        // update scene data
        let angle = self.total_seconds / 2.;
        let Vector3::<f32> { x, y, z } = vec3(angle.sin(), -1., angle.cos()).normalize();
        frame.update_scene_data(None, Some([x, y, z, 1.]), None);
    }
}
