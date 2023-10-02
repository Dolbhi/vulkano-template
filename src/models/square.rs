use crate::models::Model;
use crate::shaders::movable_square;
use crate::VertexFull;

pub struct SquareModel;

type UniformData = movable_square::vs::Data;

impl Model<VertexFull, UniformData> for SquareModel {
    fn get_vertices() -> Vec<VertexFull> {
        vec![
            VertexFull {
                position: [-0.25, -0.25, 0.0],
                normal: [0.0, 0.0, 1.0],
                colour: [0.0, 1.0, 0.0],
            },
            VertexFull {
                position: [0.25, -0.25, 0.0],
                normal: [0.0, 0.0, 1.0],
                colour: [0.0, 1.0, 0.0],
            },
            VertexFull {
                position: [-0.25, 0.25, 0.0],
                normal: [0.0, 0.0, 1.0],
                colour: [0.0, 1.0, 0.0],
            },
            VertexFull {
                position: [0.25, 0.25, 0.0],
                normal: [0.0, 0.0, 1.0],
                colour: [0.0, 1.0, 0.0],
            },
        ]
    }

    fn get_indices() -> Vec<u16> {
        vec![0, 1, 2, 1, 2, 3]
    }

    fn get_initial_uniform_data() -> UniformData {
        UniformData {
            // color: [0.0, 0.0, 0.0].into(),
            position: [0.0, 0.0],
        }
    }
}
