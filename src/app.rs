use std::time::Duration;

use winit::event_loop::EventLoop;
use winit::{event::ElementState, keyboard::KeyCode};

use crate::{game_objects::Camera, render::RenderLoop};

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
    focused: bool,
}

impl App {
    pub fn start(event_loop: &EventLoop<()>) -> Self {
        println!("Welcome to THE RUSTY RENDERER!");
        println!("Press WASD, SPACE and LSHIFT to move and Q to swap materials");

        Self {
            render_loop: RenderLoop::new(event_loop),
            camera: Camera {
                position: [0., 5., 0.].into(),
                ..Default::default()
            },
            keys: Keys::default(),
            focused: false,
        }
    }

    pub fn update(&mut self, duration_since_last_update: &Duration) {
        let seconds_passed = (duration_since_last_update.as_micros() as f32) / 1000000.0;
        // println!("Current time: {}", seconds_passed);

        self.update_movement(seconds_passed);

        self.render_loop.update(&self.camera, seconds_passed);
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

    pub fn handle_focused(&mut self, focused: bool) {
        self.focused = focused;
    }

    pub fn handle_mouse_input(&mut self, dx: f32, dy: f32) {
        if self.focused {
            self.camera.rotate(dx, dy);
        }
        // println!(
        //     "[Camera Rotation] x: {}, y: {}, z: {}",
        //     self.camera.rotation.x.0, self.camera.rotation.y.0, self.camera.rotation.z.0
        // );
    }

    pub fn handle_keyboard_input(&mut self, key_code: KeyCode, state: ElementState) {
        if !self.focused {
            return;
        }

        let state = match state {
            ElementState::Pressed => Pressed,
            ElementState::Released => Released,
        };

        match key_code {
            KeyCode::KeyQ => {
                if state == Pressed && self.keys.q == Released {
                    // self.square.change_to_random_color();
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
