use cgmath::{InnerSpace, Quaternion, Rad, Rotation, Rotation3, Vector3, Zero};

use super::{
    transform::{Transform, TransformID, TransformSystem},
    Camera, Rotate,
};
use legion::*;

pub struct Inputs {
    pub movement: Vector3<f32>,
}

impl Default for Inputs {
    fn default() -> Self {
        Self {
            movement: Vector3::zero(),
        }
    }
}

impl Inputs {
    fn move_transform(&self, transform: &mut Transform, seconds_passed: f32) {
        let view = transform.get_local_transform();

        let mut final_move = self.movement.clone();
        final_move.y = 0.;

        // if self.w == Pressed {
        //     movement.z -= 1.; // forward
        // } else if self.s == Pressed {
        //     movement.z += 1.; // backwards
        // }
        // if self.a == Pressed {
        //     movement.x -= 1.; // left
        // } else if self.d == Pressed {
        //     movement.x += 1.; // right
        // }

        final_move = view.rotation.rotate_vector(final_move);
        final_move.y = 0.;
        if final_move != Vector3::zero() {
            final_move = final_move.normalize();
        }

        final_move.y = self.movement.y;

        // apply movement
        transform.set_translation(view.translation + final_move * 2. * seconds_passed);
    }
}

pub struct GameWorld {
    pub transforms: TransformSystem,
    pub world: World,
    pub camera: Camera,
    pub fixed_seconds: f32,
    pub inputs: Inputs,
}

impl GameWorld {
    pub fn new() -> Self {
        let mut transforms = TransformSystem::new();
        let mut world = World::default();
        let camera = Camera {
            fov: Rad(1.2),
            transform: transforms.next().unwrap(),
        };
        world.push((camera.transform,));

        Self {
            transforms,
            world,
            camera,
            fixed_seconds: 0.,
            inputs: Inputs::default(),
        }
    }

    /// update world logic with a time step
    pub fn update(&mut self, seconds_passed: f32) {
        self.fixed_seconds += seconds_passed;

        // update interpolation models
        let mut query = <&TransformID>::query();
        for transform_id in query.iter(&self.world) {
            // *last_model =
            //     InterpolateTransform(self.transforms.get_global_model(transform_id).unwrap());
            if let Err(_) = self.transforms.store_last_model(transform_id) {
                println!("[Error] Failed to find transform of interpolated object");
            }
        }
        self.transforms.update_last_fixed();

        // move cam
        self.inputs.move_transform(
            self.transforms
                .get_transform_mut(&self.camera.transform)
                .unwrap(),
            seconds_passed,
        );

        // update rotate
        let mut query = <(&TransformID, &Rotate)>::query();
        for (transform_id, rotate) in query.iter(&self.world) {
            let transform = self.transforms.get_transform_mut(transform_id).unwrap();
            transform.set_rotation(
                Quaternion::from_axis_angle(rotate.0, rotate.1 * seconds_passed)
                    * transform.get_local_transform().rotation,
            );
        }
    }

    /// clear the world and transforms and reset the camera
    pub fn clear(&mut self) {
        self.world.clear();
        self.transforms = TransformSystem::new();
        self.camera = Camera {
            fov: Rad(1.2),
            transform: self.transforms.next().unwrap(),
        };
        self.world.push((self.camera.transform,));
    }
}
