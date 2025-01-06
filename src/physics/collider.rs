// mod bounds_tree;
mod bvh;

use core::f32;
use std::{
    fmt::Debug,
    sync::{Arc, RwLock},
};

use cgmath::{InnerSpace, SquareMatrix, Zero};

use bvh::{DepthIter, LeafOutsideHierachy, BVH};

use crate::game_objects::transform::{TransformID, TransformSystem};

use super::{
    contact::{Contact, ContactResolver},
    RigidBody, Vector,
};

pub use self::bvh::LeafInHierachy;
// pub type ColliderRef = Arc<Mutex<Leaf>>;

#[derive(Clone, Copy, Debug)]
pub struct BoundingBox {
    pub max: Vector,
    pub min: Vector,
}
/// cube with radius 1
pub struct CuboidCollider {
    transform: TransformID,
    rigidbody: Option<Arc<RwLock<RigidBody>>>,
    bounding_box: BoundingBox,
}
pub struct ColliderSystem {
    bounds_tree: BVH,
}

impl BoundingBox {
    /// find upper and lower bounds of given verticies
    fn from_vertices<'a>(vertices: impl IntoIterator<Item = &'a Vector>) -> Self {
        let mut vertices = vertices.into_iter();
        let mut max = *vertices.next().unwrap();
        let mut min = max;

        for vertex in vertices {
            max.x = vertex.x.max(max.x);
            max.y = vertex.y.max(max.y);
            max.z = vertex.z.max(max.z);
            min.x = vertex.x.min(min.x);
            min.y = vertex.y.min(min.y);
            min.z = vertex.z.min(min.z);
        }

        BoundingBox { max, min }
    }

    pub fn check_overlap(&self, other: Self) -> bool {
        let diff = self.centre() - other.centre();
        let extents = self.extents() + other.extents();

        diff.x.abs() < extents.x && diff.y.abs() < extents.y && diff.z.abs() < extents.z
    }

    pub fn translate(&mut self, translation: Vector) {
        self.max += translation;
        self.min += translation;
    }

    pub fn scale(&mut self, scale: f32) {
        let centre = self.centre();
        let extents = self.extents();

        self.max = centre + scale * extents;
        self.min = centre - scale * extents;
    }

    /// returns new bounds which encapsulates both input bounds
    pub fn join(self, rhs: Self) -> Self {
        let max_x = rhs.max.x.max(self.max.x);
        let max_y = rhs.max.y.max(self.max.y);
        let max_z = rhs.max.z.max(self.max.z);
        let min_x = rhs.min.x.min(self.min.x);
        let min_y = rhs.min.y.min(self.min.y);
        let min_z = rhs.min.z.min(self.min.z);

        Self {
            max: (max_x, max_y, max_z).into(),
            min: (min_x, min_y, min_z).into(),
        }
    }

    pub fn volume(&self) -> f32 {
        let extends = self.max - self.min;
        extends.x * extends.y * extends.z
    }

    pub fn centre(&self) -> Vector {
        (self.min + self.max) / 2.
    }
    pub fn extents(&self) -> Vector {
        (self.max - self.min) / 2.
    }
}
impl PartialEq for BoundingBox {
    fn eq(&self, other: &Self) -> bool {
        let max = self.max - other.max;
        let min = self.min - other.min;

        min.magnitude2() < f32::EPSILON && max.magnitude2() < f32::EPSILON
    }
}
impl Default for BoundingBox {
    fn default() -> Self {
        Self {
            max: [1., 1., 1.].into(),
            min: Vector::zero(),
        }
    }
}

/// First point is the minimal point followed by the 3 points adjacent to it
const CUBE_VERTICES: [Vector; 8] = [
    Vector {
        x: -1.0,
        y: -1.0,
        z: -1.0,
    },
    Vector {
        x: 1.0,
        y: -1.0,
        z: -1.0,
    },
    Vector {
        x: -1.0,
        y: 1.0,
        z: -1.0,
    },
    Vector {
        x: -1.0,
        y: -1.0,
        z: 1.0,
    },
    Vector {
        x: 1.0,
        y: 1.0,
        z: -1.0,
    },
    Vector {
        x: 1.0,
        y: -1.0,
        z: 1.0,
    },
    Vector {
        x: -1.0,
        y: 1.0,
        z: 1.0,
    },
    Vector {
        x: 1.0,
        y: 1.0,
        z: 1.0,
    },
];

impl CuboidCollider {
    pub fn new(
        transforms: &mut TransformSystem,
        transform: TransformID,
        rigidbody: Option<Arc<RwLock<RigidBody>>>,
    ) -> Self {
        let mut collider = CuboidCollider {
            transform,
            rigidbody,
            bounding_box: BoundingBox::default(),
        };
        collider.update_bounding(transforms);
        collider
    }

    fn update_bounding(&mut self, transforms: &mut TransformSystem) {
        let global_model = transforms.get_global_model(&self.transform).unwrap();
        // let view = transforms
        //     .get_transform(&self.transform)
        //     .unwrap()
        //     .get_local_transform();

        // let pos = global_model * Vector4::new(1.0, 0.0, 0.0, 1.0);

        // self.bounding_box.min = pos.truncate() / pos.w;
        // self.bounding_box.max = self.bounding_box.min + view.scale;

        let vertices = CUBE_VERTICES.map(|v| {
            let v = global_model * v.extend(1.0);
            v.truncate() / v.w
        });

        self.bounding_box = BoundingBox::from_vertices(&vertices);
        // self.bounding_box.translate(*view.translation);
    }

    pub fn get_bounds(&self) -> &BoundingBox {
        &self.bounding_box
    }
}
impl Debug for CuboidCollider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Collider({})", self.transform.id()))
    }
}

/// Note: Probably a useless wrapper around the bvh
impl ColliderSystem {
    pub fn new() -> Self {
        Self {
            bounds_tree: BVH::new(),
        }
    }

    pub fn tree_depth(&self) -> usize {
        self.bounds_tree.depth()
    }

    // removed update and reinsert given collider
    pub fn update(&mut self, target: &mut LeafInHierachy, transforms: &mut TransformSystem) {
        self.bounds_tree
            .modify_collider(target, |collider| collider.update_bounding(transforms))
            .unwrap();

        // // println!("[Updating bounds]");
        // let mut outside_hierachy = self.bounds_tree.remove(target).unwrap();
        // // println!("\t[Removed]");
        // let collider = outside_hierachy
        //     .get_collider_mut(&mut self.bounds_tree)
        //     .unwrap();
        // collider.update_bounding(transforms);
        // // println!("\t[Updated]");
        // self.bounds_tree.insert(outside_hierachy).unwrap()
        // // println!("\t[Inserted]");
    }

    /// adds collider to bounds tree, returns a reference to its leaf node
    pub fn add(&mut self, collider: CuboidCollider) -> LeafInHierachy {
        self.bounds_tree.insert(BVH::register_collider(collider))
    }
    pub fn remove(
        &mut self,
        target: LeafInHierachy,
    ) -> Result<LeafOutsideHierachy, LeafInHierachy> {
        self.bounds_tree.remove(target)
    }

    pub fn bounds_iter(&self) -> DepthIter {
        self.bounds_tree.iter()
    }

    pub fn get_potential_overlaps(&self) -> Vec<(&CuboidCollider, &CuboidCollider)> {
        self.bounds_tree.get_overlaps()
    }

    pub fn get_contacts(&self, transforms: &mut TransformSystem) -> ContactResolver {
        let mut result = ContactResolver::new();

        for (mut coll_1, mut coll_2) in self.bounds_tree.get_overlaps() {
            if let Some(rb_1) = &coll_1.rigidbody {
                if let Some(rb_2) = &coll_2.rigidbody {
                    if Arc::ptr_eq(rb_1, rb_2) {
                        // ignore contacts within a rigidbody
                        continue;
                    }
                }
            } else {
                if coll_2.rigidbody.is_some() {
                    std::mem::swap(&mut coll_1, &mut coll_2);
                } else {
                    // ignore contacts not involving rigidbodies
                    continue;
                }
            }

            let model_1 = transforms.get_global_model(&coll_1.transform).unwrap(); // impl func to try get model without mut
            let model_2 = transforms.get_global_model(&coll_2.transform).unwrap();

            if model_1.w.w != 1.0 || model_2.w.w != 1.0 {
                println!("w1: {}, w2: {}", model_1.w.w, model_2.w.w);
            }

            // seperating axis
            let dist = (model_1.w - model_2.w).truncate(); // might need to normalise

            let axes = [
                model_1.x, model_1.y, model_1.z, model_2.x, model_2.y, model_2.z,
            ]
            .map(|v| v.truncate());
            let cross_axes = (0..3).flat_map(|i1| (3..6).map(move |i2| axes[i1].cross(axes[i2])));

            let mut sep_axis_found = false;
            for axis in axes.into_iter().chain(cross_axes) {
                let proj_1 =
                    axes[0].dot(axis).abs() + axes[1].dot(axis).abs() + axes[2].dot(axis).abs();
                let proj_2 =
                    axes[3].dot(axis).abs() + axes[4].dot(axis).abs() + axes[5].dot(axis).abs();

                if dist.dot(axis).abs() > proj_1 + proj_2 {
                    sep_axis_found = true;
                    break;
                }
            }

            if !sep_axis_found {
                let inv_model_1 = model_1.invert().unwrap();
                let space_2_to_space_1 = inv_model_1 * model_2;

                let model_1_sqr = [
                    model_1.x.magnitude2(),
                    model_1.y.magnitude2(),
                    model_1.z.magnitude2(),
                ];

                // p-f contacts
                let points_2 =
                    CUBE_VERTICES.map(|v| (space_2_to_space_1 * v.extend(1.)).truncate());
                let mut max_pen_pf_sqr = 0.;
                let mut contact_point_pf = [0., 0., 0.].into();
                let mut pen_axis = 0;
                for point in points_2 {
                    let x_depth = 1. - point.x.abs();
                    let y_depth = 1. - point.y.abs();
                    let z_depth = 1. - point.z.abs();

                    if x_depth < 0. || y_depth < 0. || z_depth < 0. {
                        continue;
                    }

                    // convert to world dist squared
                    let point_depth_sqr = [
                        x_depth * x_depth * model_1_sqr[0],
                        y_depth * y_depth * model_1_sqr[1],
                        z_depth * z_depth * model_1_sqr[2],
                    ];

                    // find min depth for this point
                    let min_index = if point_depth_sqr[0] < point_depth_sqr[1] {
                        if point_depth_sqr[0] < point_depth_sqr[2] {
                            // x min
                            0
                        } else {
                            // z min
                            2
                        }
                    } else {
                        if point_depth_sqr[1] < point_depth_sqr[2] {
                            // y min
                            1
                        } else {
                            // z min
                            2
                        }
                    };

                    if point_depth_sqr[min_index] > max_pen_pf_sqr {
                        max_pen_pf_sqr = point_depth_sqr[min_index];
                        contact_point_pf = point;
                        pen_axis = (1 + min_index as i32) * point.x.signum() as i32;
                    }
                }

                // f-p contacts
                let axes_2 = [
                    space_2_to_space_1.x,
                    space_2_to_space_1.y,
                    space_2_to_space_1.z,
                ]
                .map(|v| v.truncate());
                let axes_2_inv = axes_2.map(|v| v / v.magnitude2());

                let x_2_sqr = model_2.x.magnitude2();
                let y_2_sqr = model_2.y.magnitude2();
                let z_2_sqr = model_2.z.magnitude2();

                let pos_2_in_1 = space_2_to_space_1.w.truncate();
                let points_1 = CUBE_VERTICES.map(|v| v - pos_2_in_1);

                for (i, point) in points_1.into_iter().enumerate() {
                    let a2_proj = axes_2_inv.map(|a| point.dot(a));

                    let x_depth = 1. - a2_proj[0].abs();
                    let y_depth = 1. - a2_proj[1].abs();
                    let z_depth = 1. - a2_proj[2].abs();

                    if x_depth < 0. || y_depth < 0. || z_depth < 0. {
                        continue;
                    }

                    // convert to world dist squared
                    let point_depth_sqr = [
                        x_depth * x_depth * x_2_sqr,
                        y_depth * y_depth * y_2_sqr,
                        z_depth * z_depth * z_2_sqr,
                    ];

                    // find min depth for this point
                    let min_index = if point_depth_sqr[0] < point_depth_sqr[1] {
                        if point_depth_sqr[0] < point_depth_sqr[2] {
                            // x min
                            0
                        } else {
                            // z min
                            2
                        }
                    } else {
                        if point_depth_sqr[1] < point_depth_sqr[2] {
                            // y min
                            1
                        } else {
                            // z min
                            2
                        }
                    };

                    if point_depth_sqr[min_index] > max_pen_pf_sqr {
                        max_pen_pf_sqr = point_depth_sqr[min_index];
                        contact_point_pf = CUBE_VERTICES[i];
                        pen_axis = (4 + min_index as i32) * a2_proj[min_index].signum() as i32;
                    }
                }

                // e-e contacts
                let cross_indices = [[1, 2], [2, 0], [0, 1]];

                let mut max_pen_ee_sqr = 0.;
                let mut contact_point_ee = [0., 0., 0.].into();
                let mut pen_axis_1 = 0;
                let mut pen_axis_2 = 0;
                // for each unique axis point on 2
                for point in [1, 2, 3, 7].map(|i| points_2[i]) {
                    // for each edge from that point
                    for (i, a2) in axes_2.iter().enumerate() {
                        // closest point on edge to 1's centre
                        let d = point - a2.dot(point) * axes_2_inv[i];

                        // closest vertex on 1 to d (closest vertex to edge)
                        let p1: Vector = [d.x.signum(), d.y.signum(), d.z.signum()].into();
                        let p1_p2 = point - p1;
                        // let test = p1_p2.mul_element_wise(*a);

                        // project edge onto each x,y,z plane
                        let a_projs: [Vector; 3] = [
                            [0., a2.y, a2.z].into(),
                            [a2.x, 0., a2.z].into(),
                            [a2.x, a2.y, 0.].into(),
                        ];
                        // get closest point of edge to 3 edges of p1
                        let d2_per_edge = a_projs
                            .map(|a_proj| point - a_proj.dot(p1_p2) * a2 / a_proj.magnitude2());

                        let mut potential_pen = None;
                        let mut potential_index = None;

                        // for each closest point
                        for a1_i in 0..3 {
                            // check if point is in 2
                            let d2_from_2 = d2_per_edge[a1_i] - space_2_to_space_1.w.truncate();
                            if d2_from_2.dot(axes_2_inv[i]).abs() > 1. {
                                continue;
                            }

                            // check if closest point on edge is in 2
                            let mut d1_from_2 = d2_from_2;
                            d1_from_2[a1_i] += p1[a1_i] - d2_per_edge[a1_i][a1_i];
                            if d1_from_2.dot(axes_2_inv[i]).abs() > 1. {
                                continue;
                            }

                            // check if point is in 1
                            let d_abs = [
                                d2_per_edge[a1_i].x.abs(),
                                d2_per_edge[a1_i].y.abs(),
                                d2_per_edge[a1_i].z.abs(),
                            ];
                            if d_abs[0] > 1. || d_abs[1] > 1. || d_abs[2] > 1. {
                                continue;
                            }

                            let [ci_1, ci_2] = cross_indices[a1_i];
                            let depth_1 = 1. - d_abs[ci_1];
                            let depth_2 = 1. - d_abs[ci_2];

                            let depth = depth_1 * depth_1 * model_1_sqr[ci_1]
                                + depth_2 * depth_2 * model_1_sqr[ci_2];

                            if let Some(min_depth) = potential_pen {
                                if depth < min_depth {
                                    potential_pen = Some(depth);
                                    potential_index = Some(a1_i)
                                }
                            } else {
                                potential_pen = Some(depth);
                                potential_index = Some(a1_i);
                            }
                        }

                        if let Some(depth) = potential_pen {
                            if depth > max_pen_ee_sqr {
                                let index = potential_index.unwrap();

                                max_pen_ee_sqr = depth;
                                contact_point_ee = d2_per_edge[index];
                                pen_axis_1 = index + 1;
                                pen_axis_2 = i + 4;
                            }
                        }
                    }
                }

                // compare p-f and e-e contacts
                // returned normal points outward from coll_1
                // should return max pen as well
                if max_pen_pf_sqr == 0. && max_pen_ee_sqr == 0.0 {
                    println!("CANT FIND CONTACT >:(");
                    continue;
                }
                if max_pen_pf_sqr >= max_pen_ee_sqr {
                    let normal = pen_axis.signum() as f32 * axes[pen_axis.abs() as usize - 1];
                    // println!("pf collision normal: {:?}", normal);
                    let point = model_1 * contact_point_pf.extend(1.);

                    let (index, contact) = Contact::new(
                        &transforms,
                        point.truncate(),
                        normal.normalize(),
                        max_pen_pf_sqr.sqrt(),
                        coll_1.rigidbody.clone().unwrap(),
                        coll_2.rigidbody.clone(),
                    );
                    result.add_contact(index, contact);
                } else {
                    let normal = axes[pen_axis_1 - 1].cross(axes[pen_axis_2 - 1]);
                    // println!("ee collision normal: {:?}", normal);
                    let point = model_1 * contact_point_ee.extend(1.);

                    let (index, contact) = Contact::new(
                        &transforms,
                        point.truncate(),
                        normal.normalize(),
                        max_pen_ee_sqr.sqrt(),
                        coll_1.rigidbody.clone().unwrap(),
                        coll_2.rigidbody.clone(),
                    );
                    result.add_contact(index, contact);
                }
            }
        }
        result
    }
}
