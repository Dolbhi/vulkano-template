pub mod app;
pub mod game_objects;
pub mod render;
pub mod shaders;
mod vertex_data;
pub mod vulkano_objects;

pub use vertex_data::{Vertex2d, Vertex3d, VertexFull};

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
