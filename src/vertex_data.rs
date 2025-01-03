use vulkano::buffer::BufferContents;
use vulkano::pipeline::graphics::vertex_input::Vertex;

#[derive(BufferContents, Vertex)]
#[repr(C)]
pub struct Vertex2d {
    #[format(R32G32_SFLOAT)]
    pub position: [f32; 2],
}

#[derive(BufferContents, Vertex)]
#[repr(C)]
pub struct Vertex3d {
    #[format(R32G32B32_SFLOAT)]
    pub position: [f32; 3],
}

#[derive(BufferContents, Vertex, Clone, Debug)]
#[repr(C)]
pub struct VertexFull {
    #[format(R32G32B32_SFLOAT)]
    pub position: [f32; 3],
    #[format(R32G32B32_SFLOAT)]
    pub normal: [f32; 3],
    #[format(R32G32B32_SFLOAT)]
    pub colour: [f32; 3],
    #[format(R32G32_SFLOAT)]
    pub uv: [f32; 2],
}

impl From<[f32; 3]> for Vertex3d {
    fn from(value: [f32; 3]) -> Self {
        Vertex3d { position: value }
    }
}
