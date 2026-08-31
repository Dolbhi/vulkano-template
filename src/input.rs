use cgmath::{InnerSpace, Rotation, Vector3, Zero};
use winit::{
    event::ElementState,
    keyboard::{KeyCode, PhysicalKey},
};

use crate::game_objects::transform::Transform;

crate::create_input_struct! {
    (w, KeyCode::KeyW),
    (a, KeyCode::KeyA),
    (s, KeyCode::KeyS),
    (d, KeyCode::KeyD),
    (space, KeyCode::Space),
    (lshift, KeyCode::ShiftLeft),
    (lctrl, KeyCode::ControlLeft),

    (q, KeyCode::KeyQ),
    (r, KeyCode::KeyR),
    (i, KeyCode::KeyI),
    (o, KeyCode::KeyO),
    (p, KeyCode::KeyP),
    (escape, KeyCode::Escape),
    (equal, KeyCode::Equal),

    lmb
}

#[derive(Clone)]
pub struct ButtonState {
    was_pressed: bool,
    /// true if last input update changed button state from released to pressed
    just_pressed: bool,
}

#[macro_export]
macro_rules! create_input_struct {
    {$(($name:ident, $code:pat)),+,$($name_2:ident),*} => {
        /// Input state is stored as a ButtonState (for rising edge detection)
        #[derive(Default, Clone)]
        pub struct InputState {
            $(pub $name: ButtonState),+,
            $(pub $name_2: ButtonState),*
        }

        impl InputState {
            /// update key state
            pub fn handle_keyboard_input(&mut self, key_code: PhysicalKey, state: ElementState) {
                match key_code {
                    $(PhysicalKey::Code($code) => {self.$name.update_state(state);},)+
                    _ => {}
                }
            }
        }
    };
}

impl InputState {
    /// FPS movement
    pub fn move_transform(
        &self,
        transform: &mut Transform,
        seconds_passed: f32,
        speed: f32,
        slow_coeff: f32,
    ) {
        let mut movement = Vector3::zero();
        let mut y_movement = 0.;
        if self.w.was_pressed {
            movement.z -= 1.; // forward
        } else if self.s.was_pressed {
            movement.z += 1.; // backwards
        }
        if self.a.was_pressed {
            movement.x -= 1.; // left
        } else if self.d.was_pressed {
            movement.x += 1.; // right
        }
        if self.space.was_pressed {
            y_movement += 1.;
        } else if self.lshift.was_pressed {
            y_movement -= 1.;
        }

        let view = transform.get_local_transform();
        movement.y = 0.;

        movement = view.rotation.rotate_vector(movement);
        movement.y = 0.;
        if !movement.is_zero() {
            movement = movement.normalize();
        }

        movement.y = y_movement;

        if self.lctrl.was_pressed {
            movement *= slow_coeff;
        }

        // apply movement
        transform.set_translation(view.translation + movement * speed * seconds_passed);
    }
}

impl ButtonState {
    fn new() -> Self {
        Self {
            was_pressed: false,
            just_pressed: false,
        }
    }

    /// Updates state and return new just_pressed
    fn update_state(&mut self, state: ElementState) -> bool {
        let pressed = state == ElementState::Pressed;
        self.just_pressed = pressed && !self.was_pressed;
        self.was_pressed = pressed;
        self.just_pressed
    }

    pub fn get_just_pressed(&self) -> bool {
        self.just_pressed
    }

    /// get button_down and reset it (kinda like an Option::take() actually)
    pub fn consume_button_down(&mut self) -> bool {
        if self.just_pressed {
            self.just_pressed = false;
            true
        } else {
            false
        }
    }
}
impl Default for ButtonState {
    fn default() -> Self {
        Self::new()
    }
}
