use std::time::Instant;

use winit::event::{DeviceEvent, Event, WindowEvent};
use winit::event_loop::EventLoop;

use vulkano_template::app::App;
// use winit::keyboard::PhysicalKey;

fn main() {
    let event_loop = EventLoop::new(); //.unwrap();
    let mut app = App::start(&event_loop);

    print!("\n\n\n\n\n\n\n\n");

    let mut previous_frame_time = Instant::now();
    let mut window_focused = false;
    event_loop.run(move |event, _elwt, control_flow| match event {
        Event::WindowEvent { event, .. } => {
            if !app.gui_update(&event) {
                match event {
                    WindowEvent::CloseRequested => {
                        control_flow.set_exit();
                        // elwt.exit();
                    }
                    WindowEvent::Resized(_) => {
                        app.handle_window_resize();
                    }
                    WindowEvent::Focused(focused) => {
                        window_focused = focused;
                    }
                    WindowEvent::KeyboardInput { input, .. } => {
                        if window_focused {
                            if let Some(code) = input.virtual_keycode {
                                app.handle_keyboard_input(code, input.state);
                            }
                            // if let PhysicalKey::Code(code) = event.physical_key {
                            //     app.handle_keyboard_input(code, event.state);
                            // }
                        }
                    }
                    _ => {}
                }
            }
        }
        Event::DeviceEvent {
            event: DeviceEvent::MouseMotion { delta },
            ..
        } => {
            if window_focused {
                app.handle_mouse_input(delta.0 as f32, delta.1 as f32);
            }
        }
        Event::RedrawRequested(_) => {
            let this_frame_time = Instant::now();
            let duration_from_last_frame = this_frame_time - previous_frame_time;

            if app.update(&duration_from_last_frame) {
                control_flow.set_exit();
            };

            previous_frame_time = this_frame_time;
        }
        Event::MainEventsCleared => app.handle_window_wait(),
        _ => (),
    })
    // .unwrap();
}
