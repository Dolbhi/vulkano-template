// mod bounds_tree;
mod bvh;
mod ray;

pub use self::bvh::LeafInHierachy;
use super::{
    contact::{Contact, ContactResolver},
    matrix_truncate, RigidBody, Vector,
};
use crate::game_objects::transform::{TransformID, TransformSystem};
use bvh::{Bvh, DepthIter, LeafOutsideHierachy};
use cgmath::{InnerSpace, Matrix, Matrix4, SquareMatrix, Zero};
use core::f32;
use ray::Ray;
use std::{
    fmt::Debug,
    sync::{Arc, RwLock, Weak},
};

const CROSS_INDICES: [[usize; 2]; 3] = [[1, 2], [2, 0], [0, 1]];

#[derive(Clone, Copy, Debug)]
pub struct BoundingBox {
    pub max: Vector,
    pub min: Vector,
}
/// cube with radius 1
pub struct CuboidCollider {
    transform: TransformID,
    rigidbody: Option<Arc<RwLock<RigidBody>>>,
    // bounding_box: BoundingBox,
}

#[derive(Default)]
pub struct ColliderSystem {
    bounds_tree: Bvh,
}

pub struct ContactIdentifier {
    pub collider: Weak<CuboidCollider>,
    element: CuboidElement,
}
pub struct ContactIdPair(pub ContactIdentifier, pub ContactIdentifier);
#[derive(PartialEq, Eq)]
enum CuboidElement {
    Vertex(u8),
    Face(u8),
    Edge(u8),
}
use CuboidElement::*;

impl BoundingBox {
    pub fn new(min: impl Into<Vector>, max: impl Into<Vector>) -> Self {
        Self {
            min: min.into(),
            max: max.into(),
        }
    }

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
        x: 1.0,
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
        // transforms: &mut TransformSystem,
        transform: TransformID,
        rigidbody: Option<Arc<RwLock<RigidBody>>>,
    ) -> Self {
        let collider = CuboidCollider {
            transform,
            rigidbody,
            // bounding_box: BoundingBox::default(),
        };
        // collider.update_bounding(transforms);
        collider
    }

    pub fn calc_bounding(&self, transforms: &mut TransformSystem) -> BoundingBox {
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

        BoundingBox::from_vertices(&vertices)
        // self.bounding_box.translate(*view.translation);
    }

    // pub fn get_bounds(&self) -> &BoundingBox {
    //     &self.bounding_box
    // }

    pub fn get_transform(&self) -> &TransformID {
        &self.transform
    }

    pub fn get_rigidbody(&self) -> &Option<Arc<RwLock<RigidBody>>> {
        &self.rigidbody
    }

    /// assuming inv_model is normalised, returned normal is not normalised
    #[allow(clippy::collapsible_else_if)]
    pub fn point_normal(point: Vector, inv_model: &Matrix4<f32>) -> Vector {
        let point_local = matrix_truncate(inv_model) * (point + inv_model.w.truncate());
        let point_abs = point_local.map(|c| c.abs());

        let axis_index = if point_abs.x >= point_abs.y {
            if point_abs.x >= point_abs.z {
                0
            } else {
                2
            }
        } else {
            if point_abs.y >= point_abs.z {
                1
            } else {
                2
            }
        };

        point_local[axis_index].signum() * inv_model.row(axis_index).truncate()
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
            bounds_tree: Bvh::new(),
        }
    }

    pub fn tree_depth(&self) -> usize {
        self.bounds_tree.depth()
    }

    // removed update and reinsert given collider
    pub fn update(&mut self, target: &mut LeafInHierachy, transforms: &mut TransformSystem) {
        self.bounds_tree
            .recalculate_bounds(target, |collider| collider.calc_bounding(transforms))
            .unwrap();
    }

    /// adds collider to bounds tree, returns a reference to its leaf node
    pub fn add(
        &mut self,
        collider: CuboidCollider,
        transforms: &mut TransformSystem,
    ) -> LeafInHierachy {
        self.bounds_tree.insert(Bvh::register_collider(
            collider.calc_bounding(transforms),
            Arc::new(collider),
        ))
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

    pub fn get_potential_overlaps(&self) -> Vec<(&Arc<CuboidCollider>, &Arc<CuboidCollider>)> {
        self.bounds_tree.get_overlaps()
    }

    pub fn raycast(
        &self,
        transforms: &mut TransformSystem,
        start: Vector,
        direction: Vector,
        distance: f32,
    ) -> Option<(Vector, &Arc<CuboidCollider>)> {
        let ray = Ray::new(start, direction, distance);
        let result = self.bounds_tree.raycast(&ray, transforms);
        result.map(|(d, c)| (ray.calc_point(d), c))
    }

    #[allow(clippy::collapsible_else_if)]
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

            // if model_1.w.w != 1.0 || model_2.w.w != 1.0 {
            //     println!(
            //         "models not normalised, w1: {}, w2: {}",
            //         model_1.w.w, model_2.w.w
            //     );
            // }

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
                let model_1_sqr = [
                    model_1.x.magnitude2(),
                    model_1.y.magnitude2(),
                    model_1.z.magnitude2(),
                ];

                let space_2_to_space_1 = inv_model_1 * model_2;
                let axes_2 = [
                    space_2_to_space_1.x,
                    space_2_to_space_1.y,
                    space_2_to_space_1.z,
                ]
                .map(|v| v.truncate());
                let axes_2_inv = axes_2.map(|v| v / v.magnitude2());
                let pos_2_in_1 = space_2_to_space_1.w.truncate();
                let model_2_sqr = [0, 1, 2].map(|i| model_2[i].magnitude2());

                // p-f contacts
                let points_2 =
                    CUBE_VERTICES.map(|v| (space_2_to_space_1 * v.extend(1.)).truncate());
                let mut max_pen_pf_sqr = 0.;
                let mut contact_point_pf = &[0., 0., 0.].into();
                let mut pen_axis = 0;
                let mut pf_elems = (Vertex(0), Face(0));
                for (p_i, point) in points_2.iter().enumerate() {
                    let depths = point.map(|c| 1. - c.abs());

                    if depths.x < 0. || depths.y < 0. || depths.z < 0. {
                        continue;
                    }

                    let mut min_pen = None;
                    let mut min_index = 0;
                    let mut min_elems = (0, 0);

                    // for each axis
                    for i in 0..3 {
                        // point projected onto closest face
                        let mut face_point = *point;
                        face_point[i] = face_point[i].signum();
                        // check if it is in 2
                        let point_from_2 = face_point - pos_2_in_1;
                        let a2_proj = axes_2_inv.map(|a| point_from_2.dot(a));
                        let depth_2 = a2_proj.map(|c| 1. - c.abs());
                        if depth_2[0] < 0. || depth_2[1] < 0. || depth_2[2] < 0. {
                            continue;
                        }

                        let depth_sqr = depths[i] * depths[i] * model_1_sqr[i];
                        if let Some(old_pen) = min_pen {
                            if depth_sqr < old_pen {
                                min_pen = Some(depth_sqr);
                                min_index = i;
                                min_elems = (i as u8, p_i as u8);
                            }
                        } else {
                            min_pen = Some(depth_sqr);
                            min_index = i;
                            min_elems = (i as u8, p_i as u8);
                        }
                    }

                    if let Some(depth) = min_pen {
                        if depth > max_pen_pf_sqr {
                            max_pen_pf_sqr = depth;
                            contact_point_pf = point;
                            pen_axis = (1 + min_index as i32) * point.x.signum() as i32;
                            pf_elems = (Face(min_elems.0), Vertex(min_elems.1));
                        }
                    };
                }

                // f-p contacts
                let points_1 = CUBE_VERTICES.map(|v| v - pos_2_in_1);
                for (i, point) in points_1.into_iter().enumerate() {
                    let a2_proj = axes_2_inv.map(|a| point.dot(a));

                    let depths = a2_proj.map(|c| 1. - c.abs());

                    if depths[0] < 0. || depths[1] < 0. || depths[2] < 0. {
                        continue;
                    }

                    let mut min_pen = None;
                    let mut min_index = 0;
                    let mut min_elems = (0, 0);
                    // for each axis
                    for j in 0..3 {
                        // // point projected onto closest face
                        // let mut face_point = point;
                        // face_point[i] = face_point[i].signum();
                        // // check if it is in 2
                        // let point_from_2 = face_point - pos_2_in_1;
                        // let a2_proj = axes_2_inv.map(|a| point_from_2.dot(a));
                        // let depth_2 = a2_proj.map(|c| 1. - c.abs());
                        // if depth_2[0] < 0. || depth_2[1] < 0. || depth_2[2] < 0. {
                        //     continue;
                        // }

                        let depth_sqr = depths[j] * depths[j] * model_2_sqr[j];
                        if let Some(old_pen) = min_pen {
                            if depth_sqr < old_pen {
                                min_pen = Some(depth_sqr);
                                min_index = j;
                                min_elems = (i as u8, j as u8);
                            }
                        } else {
                            min_pen = Some(depth_sqr);
                            min_index = j;
                            min_elems = (i as u8, j as u8);
                        }
                    }

                    if let Some(depth) = min_pen {
                        if depth > max_pen_pf_sqr {
                            max_pen_pf_sqr = depth;
                            contact_point_pf = &CUBE_VERTICES[i];
                            pen_axis = (4 + min_index as i32) * a2_proj[min_index].signum() as i32;
                            pf_elems = (Vertex(min_elems.0), Face(min_elems.1))
                        }
                    };
                }

                // e-e contacts
                let mut max_pen_ee_sqr = 0.;
                let mut contact_point_ee = [0., 0., 0.].into();
                let mut pen_axis_1 = 0;
                let mut pen_axis_2 = 0;
                let mut ee_elems = (Edge(0), Edge(0));
                // for each unique axis point on 2
                for p2_i in [1, 2, 4, 7] {
                    let point = points_2[p2_i as usize];
                    // for each edge from that point
                    for (a2_i, a2) in axes_2.iter().enumerate() {
                        // closest point on edge to 1's centre
                        let d = point - a2.dot(point) * axes_2_inv[a2_i];

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
                        let mut potential_a1_i = None;

                        // for each closest point
                        for a1_i in 0..3 {
                            // check if point is in 2
                            let d2_from_2 = d2_per_edge[a1_i] - space_2_to_space_1.w.truncate();
                            if d2_from_2.dot(axes_2_inv[a2_i]).abs() > 1. {
                                continue;
                            }

                            // check if closest point on edge is in 2
                            let mut d1_from_2 = d2_from_2;
                            d1_from_2[a1_i] += p1[a1_i] - d2_per_edge[a1_i][a1_i];
                            if d1_from_2.dot(axes_2_inv[a2_i]).abs() > 1. {
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

                            let [ci_1, ci_2] = CROSS_INDICES[a1_i];
                            let depth_1 = 1. - d_abs[ci_1];
                            let depth_2 = 1. - d_abs[ci_2];

                            let depth = depth_1 * depth_1 * model_1_sqr[ci_1]
                                + depth_2 * depth_2 * model_1_sqr[ci_2];

                            if let Some(min_depth) = potential_pen {
                                if depth < min_depth {
                                    potential_pen = Some(depth);
                                    potential_a1_i = Some(a1_i);
                                }
                            } else {
                                potential_pen = Some(depth);
                                potential_a1_i = Some(a1_i);
                            }
                        }

                        if let Some(depth) = potential_pen {
                            if depth > max_pen_ee_sqr {
                                let a1_i = potential_a1_i.unwrap();

                                max_pen_ee_sqr = depth;
                                contact_point_ee = d2_per_edge[a1_i];
                                pen_axis_1 = a1_i + 1;
                                pen_axis_2 = a2_i + 4;
                                ee_elems = (
                                    CuboidElement::from_vertex_axis(
                                        CuboidElement::closest_vertex(p1),
                                        a1_i as u8,
                                    ),
                                    CuboidElement::from_vertex_axis(p2_i, a2_i as u8),
                                )
                            }
                        }
                    }
                }

                // compare p-f and e-e contacts
                // returned normal points outward from coll_1
                // should return max pen as well
                if max_pen_pf_sqr == 0. && max_pen_ee_sqr == 0.0 {
                    // println!("CANT FIND CONTACT >:(");
                    continue;
                }
                if max_pen_pf_sqr >= max_pen_ee_sqr {
                    let normal =
                        pen_axis.signum() as f32 * axes[pen_axis.unsigned_abs() as usize - 1];
                    // println!("pf collision normal: {:?}", normal);
                    let point = model_1 * contact_point_pf.extend(1.);

                    let contact_id = ContactIdPair(
                        ContactIdentifier {
                            collider: Arc::downgrade(coll_1),
                            element: pf_elems.0,
                        },
                        ContactIdentifier {
                            collider: Arc::downgrade(coll_2),
                            element: pf_elems.1,
                        },
                    );

                    let (index, contact) = Contact::new(
                        transforms,
                        point.truncate(),
                        normal.normalize(),
                        max_pen_pf_sqr.sqrt(),
                        contact_id,
                    );
                    result.add_contact(index, contact);
                } else {
                    let normal = axes[pen_axis_1 - 1].cross(axes[pen_axis_2 - 1]);
                    // println!("ee collision normal: {:?}", normal);
                    let point = model_1 * contact_point_ee.extend(1.);

                    let contact_id = ContactIdPair(
                        ContactIdentifier {
                            collider: Arc::downgrade(coll_1),
                            element: ee_elems.0,
                        },
                        ContactIdentifier {
                            collider: Arc::downgrade(coll_2),
                            element: ee_elems.1,
                        },
                    );

                    let (index, contact) = Contact::new(
                        transforms,
                        point.truncate(),
                        normal.normalize(),
                        max_pen_ee_sqr.sqrt(),
                        contact_id,
                    );
                    result.add_contact(index, contact);
                }
            }
        }
        result
    }
}

impl PartialEq for ContactIdentifier {
    fn eq(&self, other: &Self) -> bool {
        self.collider.ptr_eq(&other.collider) && self.element == other.element
    }
}
impl PartialEq for ContactIdPair {
    fn eq(&self, other: &Self) -> bool {
        (self.0 == other.0 && self.1 == other.1) || (self.0 == other.1 && self.1 == other.0)
    }
}

impl CuboidElement {
    fn from_vertex_axis(vertex: u8, axis: u8) -> Self {
        let axis_mask = 1 << axis;
        let axis_flags = axis << 3;
        Edge(axis_flags | vertex | axis_mask)
    }

    // fn get_edge_axis(edge: u8) -> u8 {
    //     edge >> 3
    // }

    fn closest_vertex(point: Vector) -> u8 {
        let mut result = 0;
        point.map(|c| {
            result <<= 1;
            if c > 0. {
                result += 1;
            }
        });
        result
    }
}
