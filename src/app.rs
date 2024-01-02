use std::sync::Arc;
use std::time::Duration;

use cgmath::{Euler, Matrix4, Rad, Vector3, Vector4};
use legion::{IntoQuery, *};
use winit::event_loop::EventLoop;
use winit::{event::ElementState, keyboard::KeyCode};

use crate::game_objects::light::PointLightComponent;
use crate::game_objects::transform::TransformCreateInfo;
use crate::render::RenderObject;
use crate::shaders::lighting::DirectionLight;
use crate::{
    game_objects::{
        transform::{TransformID, TransformSystem},
        Camera,
    },
    render::RenderLoop,
};
use crate::{init_render_objects, MaterialSwapper};

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
    camera_transform: TransformID,
}

impl App {
    pub fn start(event_loop: &EventLoop<()>) -> Self {
        println!("Welcome to THE RUSTY RENDERER!");
        println!("Press WASD, SPACE and LSHIFT to move and Q to swap materials");

        let mut world = World::default();
        let mut transforms = TransformSystem::new();
        let mut render_loop = RenderLoop::new(event_loop);

        let suzanne = init_render_objects(
            &mut world,
            &mut transforms,
            &render_loop.renderer,
            &mut render_loop.draw_system,
        );

        let camera_transform = {
            world.push((
                transforms.add_transform(TransformCreateInfo {
                    scale: Vector3::new(0.1, 0.1, 0.1),
                    translation: Vector3::new(0., 5., -1.),
                    ..Default::default()
                }),
                PointLightComponent {
                    color: Vector4::new(1., 0., 0., 1.),
                },
            ));
            world.push((
                transforms.add_transform(TransformCreateInfo {
                    scale: Vector3::new(0.1, 0.1, 0.1),
                    translation: Vector3::new(0.0, 6.0, -1.0),
                    ..Default::default()
                }),
                PointLightComponent {
                    color: Vector4::new(0., 0., 1., 1.),
                },
            ));
            let cam_transform = transforms.next().unwrap();
            world.push((
                cam_transform,
                PointLightComponent {
                    color: Vector4::new(1., 1., 1., 1.),
                },
            ));
            cam_transform
        };

        Self {
            render_loop,
            camera: Default::default(),
            keys: Keys::default(),
            world,
            transforms,
            total_seconds: 0.,
            suzanne,
            camera_transform,
        }
    }

    pub fn update(&mut self, duration_since_last_update: &Duration) {
        let seconds_passed = duration_since_last_update.as_secs_f32();
        self.total_seconds += seconds_passed;
        // println!("Current time: {}", seconds_passed);

        // move cam
        self.update_movement(seconds_passed);
        self.transforms
            .get_transform_mut(&self.camera_transform)
            .unwrap()
            .set_translation(self.camera.position + Vector3::new(0., 0., 0.01)); // light pos cannot = cam pos else the light will glitch

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
        let mut ro_query = <&Arc<RenderObject<Matrix4<f32>>>>::query();
        let point_lights: Vec<_> = <(&TransformID, &PointLightComponent)>::query()
            .iter(&self.world)
            .map(|(t, pl)| {
                pl.clone().into_light(
                    self.transforms
                        .get_transform(t)
                        .unwrap()
                        .get_transform()
                        .translation
                        .clone(),
                )
            })
            .collect();
        // let mut dl_query = <(&TransformID, &DirectionalLightComponent)>::query();

        let angle = self.total_seconds / 4.;
        let direction = cgmath::InnerSpace::normalize(cgmath::vec3(angle.sin(), -1., angle.cos()));
        let dir = DirectionLight {
            color: [1., 1., 0., 1.],
            direction: direction.extend(1.).into(),
        };

        self.render_loop.update(
            &self.camera,
            ro_query.iter(&self.world),
            point_lights,
            [dir],
            [0.2, 0.2, 0.2, 1.],
        );
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
                        // println!("Swapped mat: {:?}", next_mat);
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

// impl<'a, P>
//     RenderUpload<
//         'a,
//         std::iter::Flatten<
//             query::ChunkIter<
//                 'a,
//                 'a,
//                 legion::Read<Arc<RenderObject<Matrix4<f32>>>>,
//                 query::EntityFilterTuple<
//                     query::ComponentFilter<Arc<RenderObject<Matrix4<f32>>>>,
//                     query::Passthrough,
//                 >,
//             >,
//         >,
//         P,
//         [DirectionLight; 1],
//     > for App
// where
//     P: IntoIterator<Item = PointLight>,
// {
//     fn get_scene_data(&self, extends: &PhysicalSize<u32>) -> crate::shaders::draw::GPUGlobalData {
//         let aspect = extends.width as f32 / extends.height as f32;
//         let proj = self.camera.projection_matrix(aspect);
//         let view = self.camera.view_matrix();
//         let view_proj = proj * view;
//         let inv_view_proj = view_proj.inverse_transform().unwrap();
//         crate::shaders::draw::GPUGlobalData {
//             view: view.into(),
//             proj: proj.into(),
//             view_proj: view_proj.into(),
//             inv_view_proj: inv_view_proj.into(),
//         }
//     }

//     fn get_render_objects(
//         &'a self,
//     ) -> std::iter::Flatten<
//         query::ChunkIter<
//             '_,
//             'a,
//             legion::Read<Arc<RenderObject<Matrix4<f32>>>>,
//             query::EntityFilterTuple<
//                 query::ComponentFilter<Arc<RenderObject<Matrix4<f32>>>>,
//                 query::Passthrough,
//             >,
//         >,
//     > {
//         let mut ro_query = <&Arc<RenderObject<Matrix4<f32>>>>::query();
//         let test = ro_query.iter(&self.world);
//         test
//     }

//     fn get_point_lights<T>(
//         &self,
//     ) -> std::iter::Map<
//         std::iter::Flatten<
//             query::ChunkIter<
//                 '_,
//                 '_,
//                 (legion::Read<TransformID>, legion::Read<PointLightComponent>),
//                 query::EntityFilterTuple<
//                     legion::query::And<(
//                         query::ComponentFilter<TransformID>,
//                         query::ComponentFilter<PointLightComponent>,
//                     )>,
//                     legion::query::And<(query::Passthrough, query::Passthrough)>,
//                 >,
//             >,
//         >,
//         T,
//     >
//     where
//         T: FnMut(
//             // (
//             //     &crate::game_objects::transform::TransformID,
//             //     &crate::game_objects::light::PointLightComponent,
//             // ),
//         ) -> (),
//     {
//         <(&TransformID, &PointLightComponent)>::query()
//             .iter(&self.world)
//             .map(|(t, pl): (_, &PointLightComponent)| {
//                 pl.clone().into_light(
//                     self.transforms
//                         .get_transform(t)
//                         .unwrap()
//                         .get_transform()
//                         .translation
//                         .clone(),
//                 )
//             })
//     }

//     fn get_direction_lights(&self) -> [DirectionLight; 1] {
//         let angle = self.total_seconds / 4.;
//         let direction = cgmath::InnerSpace::normalize(cgmath::vec3(angle.sin(), -1., angle.cos()));
//         [DirectionLight {
//             color: [1., 1., 0., 1.],
//             direction: direction.extend(1.).into(),
//         }]
//     }

//     fn get_ambient_color(&self) -> [f32; 4] {
//         [0.2, 0.2, 0.2, 1.]
//     }
// }
