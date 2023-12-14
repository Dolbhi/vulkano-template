use std::iter::zip;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use cgmath::{Euler, Matrix4, Rad};
use legion::*;
use winit::event_loop::EventLoop;
use winit::{event::ElementState, keyboard::KeyCode};

use crate::game_objects::transform::Transform;
use crate::render::mesh::from_obj;
use crate::render::{DrawSystem, MaterialID, RenderObject, Renderer};
use crate::shaders::basic::vs::GPUObjectData;
use crate::VertexFull;
use crate::{
    game_objects::{
        transform::{TransformID, TransformSystem},
        Camera,
    },
    render::RenderLoop,
};

// TO flush_next_future METHOD ADD PARAMS FOR PASSING CAMERA DESCRIPTOR SET

#[derive(Default, PartialEq)]
pub enum KeyState {
    Pressed,
    #[default]
    Released,
}

use KeyState::{Pressed, Released};

#[derive(Default)]
struct Keys {
    a: KeyState,
    w: KeyState,
    s: KeyState,
    d: KeyState,
    q: KeyState,
    space: KeyState,
    shift: KeyState,
}

pub struct App {
    render_loop: RenderLoop,
    camera: Camera,
    keys: Keys,
    world: World,
    transforms: TransformSystem,
    total_seconds: f32,
    suzanne: TransformID,
}

impl App {
    pub fn start(event_loop: &EventLoop<()>) -> Self {
        println!("Welcome to THE RUSTY RENDERER!");
        println!("Press WASD, SPACE and LSHIFT to move and Q to swap materials");

        let mut world = World::default();
        let mut transforms = TransformSystem::new();
        let mut render_loop = RenderLoop::new(event_loop);

        let suzanne = Self::init_render_objects(
            &mut world,
            &mut transforms,
            &render_loop.renderer,
            &mut render_loop.render_data,
        );

        Self {
            render_loop,
            camera: Default::default(),
            keys: Keys::default(),
            world,
            transforms,
            total_seconds: 0.,
            suzanne,
        }
    }
    fn init_render_objects(
        world: &mut World,
        transform_sys: &mut TransformSystem,
        renderer: &Renderer,
        draw_system: &mut DrawSystem<GPUObjectData, Matrix4<f32>>,
    ) -> TransformID {
        let resource_loader = renderer.get_resource_loader();
        let basic_id = 0;
        let phong_id = 1;
        let uv_id = 2;

        // Texture
        let le_texture = resource_loader.load_texture(Path::new("models/lost_empire-RGBA.png"));

        let ina_textures = [
            "models/ina/Hair_Base_Color.png",
            "models/ina/Cloth_Base_Color.png",
            "models/ina/Body_Base_Color.png",
            "models/ina/Head_Base_Color.png",
        ]
        .map(|p| resource_loader.load_texture(Path::new(p)));

        let linear_sampler = resource_loader.load_sampler(vulkano::image::sampler::Filter::Linear);

        // materials
        //  lost empire
        let le_mat_id = draw_system.add_material(
            basic_id,
            "lost_empire",
            Some(draw_system.get_pipeline(basic_id).create_material_set(
                &renderer.allocators,
                2,
                le_texture.clone(),
                linear_sampler.clone(),
            )),
        );
        let le_lit_mat_id = draw_system.add_material(
            phong_id,
            "lost_empire_lit",
            Some(draw_system.get_pipeline(phong_id).create_material_set(
                &renderer.allocators,
                2,
                le_texture,
                linear_sampler.clone(),
            )),
        );

        //  ina
        let ina_ids: Vec<_> = zip(["hair", "cloth", "body", "head"], ina_textures)
            .map(|(id, tex)| {
                draw_system.add_material(
                    phong_id,
                    id,
                    Some(draw_system.get_pipeline(phong_id).create_material_set(
                        &renderer.allocators,
                        2,
                        tex,
                        linear_sampler.clone(),
                    )),
                )
            })
            .collect();

        //  uv
        let uv_mat_id = draw_system.add_material(uv_id, "uv", None);

        // meshes
        //      suzanne
        let (vertices, indices) = from_obj(Path::new("models/suzanne.obj")).pop().unwrap();
        let suzanne_mesh = resource_loader.load_mesh(vertices, indices);

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
        let square = resource_loader.load_mesh(vertices, indices);

        //      lost empire
        let le_meshes: Vec<_> = from_obj(Path::new("models/lost_empire.obj"))
            .into_iter()
            .map(|(vertices, indices)| resource_loader.load_mesh(vertices, indices))
            .collect();

        //      ina
        let ina_meshes: Vec<_> = from_obj(Path::new("models/ina/ReadyToRigINA.obj"))
            .into_iter()
            .skip(2)
            .map(|(vertices, indices)| resource_loader.load_mesh(vertices, indices))
            .collect();

        println!("[Rendering Data]");
        println!("Lost empire mesh count: {}", le_meshes.len());
        println!("Ina mesh count: {}", ina_meshes.len());

        // objects
        //  Suzanne
        let suzanne_obj = Arc::new(RenderObject::new(suzanne_mesh, uv_mat_id.clone()));
        let suzanne = transform_sys.next().unwrap();
        world.push((suzanne, suzanne_obj));

        //  Squares
        for (x, y, z) in [(1., 0., 0.), (0., 1., 0.), (0., 0., 1.)] {
            let square_obj = Arc::new(RenderObject::new(square.clone(), uv_mat_id.clone()));
            let mut transform = Transform::default();
            transform.set_translation([x, y, z]);

            world.push((transform_sys.add_transform(transform), square_obj));
        }

        //  Ina
        let mut ina_transform = Transform::default();
        ina_transform.set_translation([0.0, 5.0, -1.0]);
        let ina_transform = transform_sys.add_transform(ina_transform);
        // world.push((ina_transform));

        for (mesh, mat_id) in zip(ina_meshes, ina_ids.clone()) {
            let obj = Arc::new(RenderObject::new(mesh, mat_id));
            let mut transform = Transform::default();
            transform.set_parent(ina_transform);

            world.push((transform_sys.add_transform(transform), obj));
        }

        //  lost empires
        let le_transform = transform_sys.add_transform(Transform::default());
        for mesh in le_meshes {
            let le_obj = Arc::new(RenderObject::new(mesh, le_mat_id.clone()));
            let mut transform = Transform::default();
            transform.set_parent(le_transform);

            let mat_swapper = MaterialSwapper::new([
                le_mat_id.clone(),
                le_lit_mat_id.clone(),
                uv_mat_id.clone(),
                "cloth".into(),
            ]);

            world.push((transform_sys.add_transform(transform), le_obj, mat_swapper));
        }

        suzanne
    }

    pub fn update(&mut self, duration_since_last_update: &Duration) {
        let seconds_passed = duration_since_last_update.as_secs_f32();
        self.total_seconds += seconds_passed;
        // println!("Current time: {}", seconds_passed);

        // move cam
        self.update_movement(seconds_passed);

        // rotate suzanne
        self.transforms
            .get_transform_mut(&self.suzanne)
            .unwrap()
            .set_rotation(Euler {
                x: Rad(0.),
                y: Rad(self.total_seconds),
                z: Rad(0.),
            });

        // update render objects
        let mut query = <(&TransformID, &mut Arc<RenderObject<Matrix4<f32>>>)>::query();
        // println!("==== RENDER OBJECT DATA ====");
        for (transform_id, render_object) in query.iter_mut(&mut self.world) {
            // update object data
            match Arc::get_mut(render_object) {
                Some(obj) => {
                    let transfrom_matrix = self.transforms.get_global_model(transform_id);
                    // println!("Obj {:?}: {:?}", transform_id, obj);
                    obj.set_matrix(transfrom_matrix)
                    // obj.update_transform([0., 0., 0.], cgmath::Rad(self.total_seconds));
                }
                None => {
                    panic!("Unable to update render object");
                }
            }
        }
        // query render objects
        let mut query = <&Arc<RenderObject<Matrix4<f32>>>>::query();

        self.render_loop
            .update(&self.camera, query.iter_mut(&mut self.world));
    }

    fn update_movement(&mut self, seconds_passed: f32) {
        if self.keys.space == Pressed && self.keys.shift == Released {
            self.camera.move_up(seconds_passed)
        }
        if self.keys.shift == Pressed && self.keys.space == Released {
            self.camera.move_down(seconds_passed)
        }

        if self.keys.w == Pressed && self.keys.s == Released {
            self.camera.move_forward(seconds_passed)
        }
        if self.keys.s == Pressed && self.keys.w == Released {
            self.camera.move_back(seconds_passed)
        }

        if self.keys.a == Pressed && self.keys.d == Released {
            self.camera.move_left(seconds_passed)
        }
        if self.keys.d == Pressed && self.keys.a == Released {
            self.camera.move_right(seconds_passed)
        }
    }

    pub fn handle_mouse_input(&mut self, dx: f32, dy: f32) {
        self.camera.rotate(dx, dy);

        // println!(
        //     "[Camera Rotation] x: {}, y: {}, z: {}",
        //     self.camera.rotation.x.0, self.camera.rotation.y.0, self.camera.rotation.z.0
        // );
    }
    pub fn handle_keyboard_input(&mut self, key_code: KeyCode, state: ElementState) {
        let state = match state {
            ElementState::Pressed => Pressed,
            ElementState::Released => Released,
        };

        match key_code {
            KeyCode::KeyQ => {
                if state == Pressed && self.keys.q == Released {
                    let mut query =
                        <(&mut MaterialSwapper, &mut Arc<RenderObject<Matrix4<f32>>>)>::query();

                    query.for_each_mut(&mut self.world, |(swapper, render_object)| {
                        let next_mat = swapper.swap_material();
                        println!("Swapped mat: {:?}", next_mat);
                        match Arc::get_mut(render_object) {
                            Some(obj) => {
                                obj.material_id = next_mat;
                            }
                            None => {
                                panic!("Unable to swap material on render object");
                            }
                        }
                    });
                }
                self.keys.q = state;
            }
            KeyCode::KeyW => self.keys.w = state,
            KeyCode::KeyA => self.keys.a = state,
            KeyCode::KeyS => self.keys.s = state,
            KeyCode::KeyD => self.keys.d = state,
            KeyCode::Space => self.keys.space = state,
            KeyCode::ShiftLeft => self.keys.shift = state,
            _ => {}
        }
    }

    pub fn handle_window_resize(&mut self) {
        self.render_loop.handle_window_resize()
    }
    pub fn handle_window_wait(&self) {
        self.render_loop.handle_window_wait();
    }
}

struct MaterialSwapper {
    materials: Vec<MaterialID>,
    curent_index: usize,
}
impl MaterialSwapper {
    fn new(materials: impl IntoIterator<Item = impl Into<MaterialID>>) -> Self {
        let materials = materials.into_iter().map(|m| m.into()).collect();
        Self {
            materials,
            curent_index: 0,
        }
    }

    fn swap_material(&mut self) -> MaterialID {
        self.curent_index = (self.curent_index + 1) % self.materials.len();
        self.materials[self.curent_index].clone()
    }
}
