use std::ops::Neg;

use cgmath::{BaseFloat, BaseNum, Matrix3, Matrix4, One, Vector3, num_traits::Num};

/// Create an antisymmetric skew matrix with the given vector components
pub fn skew<S: Num + Neg<Output = S> + Copy>(v: Vector3<S>) -> Matrix3<S> {
    Matrix3 {
        x: (S::zero(), v.z, -v.y).into(),
        y: (-v.z, S::zero(), v.x).into(),
        z: (v.y, -v.x, S::zero()).into(),
    }
}

pub fn apply_model<S: BaseFloat + One>(m: Matrix4<S>, v: Vector3<S>) -> Vector3<S> {
    (m * v.extend(S::one())).truncate()
}

/// Split a transformation model into scale * rotation and translation
pub fn split_model<S: BaseNum>(m: Matrix4<S>) -> (Matrix3<S>, Vector3<S>) {
    (Matrix3::from_cols(m.x.truncate(), m.y.truncate(), m.z.truncate()), m.w.truncate())
}