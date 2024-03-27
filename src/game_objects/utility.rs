use cgmath::{Vector3, VectorSpace};

pub fn smooth_follow(
    current: Vector3<f32>,
    target: Vector3<f32>,
    smooth_speed: f32,
) -> Vector3<f32> {
    current.lerp(target, smooth_speed)
}
