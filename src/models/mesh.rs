use crate::VertexFull;

pub struct Mesh {
    vertices: Vec<VertexFull>,
    indicies: Vec<u16>,
}

impl Mesh {
    pub fn get_vertices(&self) -> Vec<VertexFull> {
        return self.vertices;
    }

    pub fn get_indicies(&self) -> Vec<u16> {
        return self.indicies;
    }
}
