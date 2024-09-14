pub mod app;
pub mod game_objects;
mod physics;
pub mod prefabs;
pub mod profiler;
pub mod render;
pub mod shaders;
pub mod ui;
mod vertex_data;
pub mod vulkano_objects;

use std::sync::Mutex;

pub use vertex_data::{Vertex2d, Vertex3d, VertexFull};

use profiler::Profiler;

pub static mut RENDER_PROFILER: Option<Profiler<7, 128>> = Some(Profiler::new([
    "GUI and inputs",
    "Pre-render",
    "Frame cleanup",
    "Render upload",
    "Wait last frame",
    "ComBuf building",
    "Execute",
]));
pub static mut LOGIC_PROFILER: Mutex<Profiler<5, 128>> = Mutex::new(Profiler::new([
    "Lock wait",
    "Colliders",
    "Physics",
    "Interpolate",
    "Others",
]));

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
