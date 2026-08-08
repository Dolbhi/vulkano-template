use winit::event_loop::EventLoop;

use vulkano_template::app::App;
// use winit::keyboard::PhysicalKey;

fn main() {
    let event_loop = EventLoop::new().unwrap(); //.unwrap();
    let mut app = App::start();

    let _ = event_loop.run_app(&mut app);
}
