use super::GPUColoredData;
use cgmath::{Matrix, Matrix4, Transform};
use winit::dpi::PhysicalSize;

pub struct ColoredData {
    transform: Matrix4<f32>,
    color: Vector4,
}

impl From<ColoredData> for GPUColoredData {
    fn from(value: ColoredData) -> Self {
        GPUColoredData {
            render_matrix: value.transform.into(),
            normal_matrix: value
                .transform
                .inverse_transform()
                .unwrap()
                .transpose()
                .into(),
            color: value.color.into(),
        }
    }
}
