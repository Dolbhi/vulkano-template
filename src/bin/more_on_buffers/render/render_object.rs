use cgmath::{Matrix4, SquareMatrix};

use vulkano::buffer::BufferContents;
use vulkano_template::{
    game_objects::Square, shaders::movable_square::vs::Data, vulkano_objects::buffers::Uniform,
};

pub struct RenderObject<U: BufferContents + Clone> {
    // buffers: Buffers,
    pub mesh_id: String,
    pub pipeline_id: String,
    uniforms: Vec<Uniform<U>>,
    transform_matrix: Matrix4<f32>,
}

impl<U: BufferContents + Clone> RenderObject<U> {
    pub fn new(mesh_id: String, pipeline_id: String, uniforms: Vec<Uniform<U>>) -> Self {
        Self {
            mesh_id,
            pipeline_id,
            uniforms,
            transform_matrix: Matrix4::identity(),
        }
    }

    // pub fn get_buffers(&self) -> &Buffers {
    //     &self.buffers
    // }

    pub fn get_uniforms(&self) -> &Vec<Uniform<U>> {
        &self.uniforms
    }
}

impl RenderObject<Data> {
    pub fn update_uniform(&self, index: u32, square: &Square, radians: cgmath::Rad<f32>) {
        let mut uniform_content = self.uniforms[index as usize]
            .0
            .write()
            .unwrap_or_else(|e| panic!("Failed to write to uniform buffer\n{}", e));

        // uniform_content.color = square.color.into();
        // uniform_content.position = square.position.into();

        let cam_pos = cgmath::vec3(0., 0., 2.);
        let view = Matrix4::from_translation(-cam_pos);
        let projection = cgmath::perspective(cgmath::Rad(1.2), 1., 0.1, 200.);
        // projection.y.y *= -1.;
        let model = Matrix4::from_axis_angle(cgmath::vec3(0., 1., 0.), radians);

        let translation =
            Matrix4::from_translation([square.position[0], square.position[1], 0.].into());

        uniform_content.render_matrix = (projection * view * model * translation).into();
    }
}
