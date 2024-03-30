use cgmath::{InnerSpace, Zero};
use std::ops::{Add, Mul};

use super::Vector;

#[derive(Clone, Copy)]
struct FullMultiVector {
    s: f32,
    v: Vector,
    b: Vector,
    p: f32,
}
impl FullMultiVector {
    fn zero() -> Self {
        Self {
            s: 0.,
            v: Vector::zero(),
            b: Vector::zero(),
            p: 0.,
        }
    }
}
impl From<MultiVectorComponent> for FullMultiVector {
    fn from(value: MultiVectorComponent) -> Self {
        match value {
            S(s) => FullMultiVector {
                s,
                ..FullMultiVector::zero()
            },
            V(v) => FullMultiVector {
                v,
                ..FullMultiVector::zero()
            },
            B(b) => FullMultiVector {
                b,
                ..FullMultiVector::zero()
            },
            P(p) => FullMultiVector {
                p,
                ..FullMultiVector::zero()
            },
            Even(HalfMultiVector { s, v }) => FullMultiVector {
                s,
                b: v,
                ..FullMultiVector::zero()
            },
            Odd(HalfMultiVector { s, v }) => FullMultiVector {
                p: s,
                v,
                ..FullMultiVector::zero()
            },
            Full(v) => v,
        }
    }
}

#[derive(Clone, Copy)]
struct HalfMultiVector {
    s: f32,
    v: Vector,
}

#[derive(Clone, Copy)]
pub enum MultiVectorComponent {
    S(f32),
    V(Vector),
    B(Vector),
    P(f32),
    Even(HalfMultiVector),
    Odd(HalfMultiVector),
    Full(FullMultiVector),
}
use MultiVectorComponent::*;

impl MultiVectorComponent {}
impl Add for MultiVectorComponent {
    type Output = MultiVectorComponent;

    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Full(l), S(r)) => Full(FullMultiVector { s: l.s + r, ..l }),
            (Full(l), V(r)) => Full(FullMultiVector { v: l.v + r, ..l }),
            (Full(l), B(r)) => Full(FullMultiVector { b: l.b + r, ..l }),
            (Full(l), P(r)) => Full(FullMultiVector { p: l.p + r, ..l }),
            (Full(l), Even(r)) => Full(FullMultiVector {
                s: l.s + r.s,
                b: l.b + r.v,
                ..l
            }),
            (Full(l), Odd(r)) => Full(FullMultiVector {
                p: l.p + r.s,
                v: l.v + r.v,
                ..l
            }),

            (S(l), S(r)) => S(l + r),
            (V(l), V(r)) => V(l + r),
            (B(l), B(r)) => B(l + r),
            (P(l), P(r)) => P(l + r),
            (Even(l), Even(r)) => Even(HalfMultiVector {
                s: l.s + r.s,
                v: l.v + r.v,
            }),
            (Odd(l), Odd(r)) => Odd(HalfMultiVector {
                s: l.s + r.s,
                v: l.v + r.v,
            }),
            (Full(l), Full(r)) => Full(FullMultiVector {
                s: l.s + r.s,
                v: l.v + r.v,
                b: l.b + r.b,
                p: l.p + r.p,
            }),

            (_, Full { .. }) => rhs + self,
            (_, Even { .. }) => rhs + self,
            (_, Odd { .. }) => rhs + self,
            (l, r) => r + l,
        }
    }
}
// impl Add<FullMultiVector> for MultiVectorComponent {
//     type Output = FullMultiVector;

//     fn add(self, rhs: FullMultiVector) -> Self::Output {
//         match self {
//             S(value) => FullMultiVector {
//                 s: value + rhs.s,
//                 ..rhs
//             },
//             V(value) => FullMultiVector {
//                 v: value + rhs.v,
//                 ..rhs
//             },
//             B(value) => FullMultiVector {
//                 b: value + rhs.b,
//                 ..rhs
//             },
//             P(value) => FullMultiVector {
//                 p: value + rhs.p,
//                 ..rhs
//             },
//         }
//     }
// }
// impl Mul for MultiVectorComponent {
//     type Output = Vec<MultiVectorComponent>;

//     fn mul(self, rhs: Self) -> Self::Output {
//         match (self, rhs) {
//             (S(l), S(r)) => vec![S(l * r)],
//             (S(s), V(v)) | (V(v), S(s)) => vec![V(s * v)],
//             (S(s), B(v)) | (B(v), S(s)) => vec![B(s * v)],
//             (S(s), P(v)) | (P(v), S(s)) => vec![P(s * v)],

//             (P(l), P(r)) => vec![P(-l * r)],
//             (P(s), V(v)) | (V(v), P(s)) => vec![B(s * v)],
//             (P(s), B(v)) | (B(v), P(s)) => vec![V(-s * v)],

//             (V(l), V(r)) => vec![S(l.dot(r)), B(l.cross(r))],
//             (V(l), B(r)) | (B(l), V(r)) => vec![V(l.cross(r)), P(l.dot(r))],

//             (B(l), B(r)) => vec![S(-l.dot(r)), B(-l.cross(r))],
//         }
//     }
// }
