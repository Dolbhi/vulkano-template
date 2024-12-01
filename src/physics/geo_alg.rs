//! Canonical basis:
//!
//! 1
//!
//! x,  y,  z
//!
//! ix, iy, iz  =   yz, zx, xy
//!
//! i           =   xyz

use std::ops::{Add, Mul};

use cgmath::{InnerSpace, Quaternion, Zero};

use super::Vector;

// fn complex_sqrt(r: f32, i: f32) -> (f32, f32, f32) {
//     let l = (r * r + i * i).sqrt();
//     // let sign = i.signum();

//     let a2 = (l + r) / 2.0;
//     let b2 = (l - r) / 2.0;

//     (a2, b2, l)
// }

#[allow(unused)]
pub fn geo_prod(lhs: Vector, rhs: Vector) -> FullMultiVector {
    FullMultiVector {
        s: lhs.dot(rhs),
        b: (
            lhs.y * rhs.z - lhs.z * lhs.y,
            lhs.z * rhs.x - lhs.x * rhs.z,
            lhs.x * rhs.y - lhs.y * rhs.x,
        )
            .into(),
        ..FullMultiVector::zero()
    }
}
#[allow(unused)]
pub fn vec_exp(vec: Vector) -> FullMultiVector {
    let l = vec.magnitude();
    let v = vec / l;
    FullMultiVector {
        s: l.cosh(),
        v: v * l.sinh(),
        ..FullMultiVector::zero()
    }
}
pub fn bivec_exp(bivec: Vector) -> FullMultiVector {
    let l = bivec.magnitude();
    let b = bivec / l;
    FullMultiVector {
        s: l.cos(),
        b: b * l.sin(),
        ..FullMultiVector::zero()
    }
}

/// Element of 3D geometric algebra
#[derive(Clone, Copy)]
pub struct FullMultiVector {
    /// Scalar component
    pub s: f32,
    /// Vector component
    pub v: Vector,
    /// Bivector component
    pub b: Vector,
    /// Psudoscalar component
    pub p: f32,
}

impl FullMultiVector {
    pub fn zero() -> Self {
        FullMultiVector {
            s: 0.,
            v: Vector::zero(),
            b: Vector::zero(),
            p: 0.,
        }
    }
    /// Returns the multivector mulitplied by the unit psudoscalar (i)
    pub fn psudo_conjugate(self) -> Self {
        Self {
            s: -self.p,
            v: -self.b,
            b: self.v,
            p: self.s,
        }
    }
    pub fn into_quaternion(self) -> Quaternion<f32> {
        Quaternion {
            s: self.s,
            v: self.b,
        }
    }
    // pub fn exp(&self) -> Self {
    //     let es = self.s.exp();
    //     let (r, i, l) = complex_sqrt(
    //         self.v.magnitude2() - self.b.magnitude2(),
    //         2.0 * self.v.dot(self.b),
    //     );

    //     todo!()
    // }
}

impl From<Vector> for FullMultiVector {
    fn from(value: Vector) -> Self {
        Self {
            v: value,
            ..FullMultiVector::zero()
        }
    }
}
impl From<Quaternion<f32>> for FullMultiVector {
    fn from(value: Quaternion<f32>) -> Self {
        Self {
            s: value.s,
            v: value.v,
            ..Self::zero()
        }
    }
}

impl Add for FullMultiVector {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self {
            s: self.s + rhs.s,
            v: self.v + rhs.v,
            b: self.b + rhs.b,
            p: self.p + rhs.p,
        }
    }
}
impl Add<f32> for FullMultiVector {
    type Output = Self;

    fn add(self, rhs: f32) -> Self {
        Self {
            s: self.s + rhs,
            ..self
        }
    }
}
impl Add<FullMultiVector> for f32 {
    type Output = FullMultiVector;

    fn add(self, rhs: FullMultiVector) -> FullMultiVector {
        rhs + self
    }
}
impl Add<Vector> for FullMultiVector {
    type Output = Self;

    fn add(self, rhs: Vector) -> Self {
        Self {
            v: self.v + rhs,
            ..self
        }
    }
}
impl Add<FullMultiVector> for Vector {
    type Output = FullMultiVector;

    fn add(self, rhs: FullMultiVector) -> FullMultiVector {
        rhs + self
    }
}

impl Mul<f32> for FullMultiVector {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self {
        Self {
            s: self.s * rhs,
            v: self.v * rhs,
            b: self.b * rhs,
            p: self.p * rhs,
        }
    }
}
impl Mul<FullMultiVector> for f32 {
    type Output = FullMultiVector;

    fn mul(self, rhs: FullMultiVector) -> FullMultiVector {
        rhs * self
    }
}

impl Mul for FullMultiVector {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        Self {
            s: self.s * rhs.s + self.v.dot(rhs.v) - self.b.dot(rhs.b) - self.p * rhs.p,
            v: self.s * rhs.v + self.v * rhs.s - self.b * rhs.p - self.p * rhs.b,
            b: self.s * rhs.b + self.v * rhs.p + self.b * rhs.s + self.p * rhs.v,
            p: self.s * rhs.p + self.v.dot(rhs.b) + self.b.dot(rhs.v) + self.p * rhs.s,
        }
    }
}
