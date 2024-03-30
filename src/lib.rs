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
pub static mut LOGIC_PROFILER: Option<Profiler<2, 128>> =
    Some(Profiler::new(["Lock wait", "Logic update"]));

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
