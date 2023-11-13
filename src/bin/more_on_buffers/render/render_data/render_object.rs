use cgmath::{Matrix4, Rad, SquareMatrix};

pub struct RenderObject {
    pub mesh_id: String,
    pub material_id: String,
    transform: Matrix4<f32>,
}

impl RenderObject {
    pub fn new(mesh_id: String, material_id: String) -> Self {
        Self {
            mesh_id,
            material_id,
            // uniforms,
            transform: Matrix4::identity(),
        }
    }

    pub fn get_transform_matrix(&self) -> Matrix4<f32> {
        self.transform
    }

    pub fn update_transform(&mut self, position: [f32; 3], rotation: Rad<f32>) {
        let rotation = Matrix4::from_axis_angle([0., 1., 0.].into(), rotation);
        let translation = Matrix4::from_translation(position.into());

        self.transform = translation * rotation;
    }
}
