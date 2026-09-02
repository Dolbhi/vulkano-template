use std::ops::Neg;

use cgmath::{num_traits::Num, Matrix3, Vector3};

/// Create an antisymmetric skew matrix with the given vector components
pub fn skew<S: Num + Neg<Output = S> + Copy>(v: Vector3<S>) -> Matrix3<S> {
    Matrix3 {
        x: (S::zero(), v.z, -v.y).into(),
        y: (-v.z, S::zero(), v.x).into(),
        z: (v.y, -v.x, S::zero()).into(),
    }
}
