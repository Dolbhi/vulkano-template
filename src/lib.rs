pub mod app;
pub mod game_objects;
pub mod prefabs;
pub mod profiler;
pub mod render;
pub mod shaders;
pub mod ui;
mod vertex_data;
pub mod vulkano_objects;

pub use vertex_data::{Vertex2d, Vertex3d, VertexFull};

use profiler::Profiler;
use render::RenderSubmit;

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

pub struct MaterialSwapper {
    materials: Vec<RenderSubmit>,
    curent_index: usize,
}
impl MaterialSwapper {
    fn new(materials: impl IntoIterator<Item = RenderSubmit>) -> Self {
        let materials = materials.into_iter().map(|m| m.into()).collect();
        Self {
            materials,
            curent_index: 0,
        }
    }

    pub fn swap_material(&mut self) -> RenderSubmit {
        self.curent_index = (self.curent_index + 1) % self.materials.len();
        self.materials[self.curent_index].clone()
    }
}
