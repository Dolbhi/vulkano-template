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
use cgmath::{InnerSpace, Matrix, Matrix4, MetricSpace, SquareMatrix, Zero};
use core::f32;
use ray::Ray;
use std::{
    fmt::Debug,
    sync::{Arc, RwLock, Weak},
};

const CROSS_INDICES: [[usize; 2]; 3] = [[1, 2], [2, 0], [0, 1]];
/// allow resolving velocity of contacts close to penetrating, penetration resolution won't happen if it remains negative
const CACHED_NEG_DEPTH: f32 = -0.1;

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
    contact_resolver: ContactResolver,
}

#[derive(Debug)]
pub struct ContactIdentifier {
    pub collider: Weak<CuboidCollider>,
    element: CuboidElement,
}
#[derive(Debug)]
pub struct ContactIdPair(pub ContactIdentifier, pub ContactIdentifier);

/// Vertex: bits 0, 1 and 2 correspond to the x, y and z components of the vertex where 0 => -1.0 and 1 => 1.0
///
/// Face: value corresponds to normal axis of face, 0 => x-axis, 1 => y-axis, 2 => z-axis
#[derive(PartialEq, Eq, Debug)]
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

/// First point is the minimal point, permutating in a binary style
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
            contact_resolver: ContactResolver::new(),
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
    pub fn get_last_contacts(&self) -> &Vec<(Vector, Vector, u8)> {
        &self.contact_resolver.past_contacts
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
    pub fn get_contacts(&mut self, transforms: &mut TransformSystem) -> &mut ContactResolver {
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

                let unit_bounds = BoundingBox {
                    min: CUBE_VERTICES[0],
                    max: CUBE_VERTICES[7],
                };

                let mut max_pen_pf_sqr = 0.;
                let mut contact_point_pf = [0., 0., 0.].into();
                let mut pen_axis = 0;
                let mut pf_elems = (Vertex(0), Face(0));
                // p-f contacts
                let points_2 =
                    CUBE_VERTICES.map(|v| (space_2_to_space_1 * v.extend(1.)).truncate());
                for p2_i in 0..4 {
                    let p2_min = points_2[p2_i];
                    let p2_max = points_2[7 - p2_i];
                    let ray = Ray {
                        origin: p2_min,
                        direction: p2_max - p2_min,
                        distance: 1.,
                    };
                    let (close, far) = ray.box_intersection_raw(&unit_bounds);

                    if close <= far && far >= 0. && close <= 1. {
                        let min_depth = far; // min point pen into far face
                        let max_depth = 1. - close; // max point pen into close face

                        // use smaller depth
                        if min_depth > max_depth {
                            let p1 = ray.calc_point(close);
                            let a1_i = CuboidElement::closest_face(p1);
                            let d_local = 1. - p2_max[a1_i as usize].abs(); // should be +ve i swear
                            let depth_sqr = d_local * d_local * model_1_sqr[a1_i as usize];

                            if depth_sqr > max_pen_pf_sqr {
                                max_pen_pf_sqr = depth_sqr;
                                contact_point_pf = p2_max;
                                pen_axis = a1_i + 1;
                                pf_elems = (Face(a1_i), Vertex(7 - p2_i as u8));
                                // use max index for p2
                            }
                        } else {
                            let p1 = ray.calc_point(far);
                            let a1_i = CuboidElement::closest_face(p1);
                            let d_local = 1. - p2_min[a1_i as usize].abs(); // should be +ve i swear
                            let depth_sqr = d_local * d_local * model_1_sqr[a1_i as usize];

                            if depth_sqr > max_pen_pf_sqr {
                                max_pen_pf_sqr = depth_sqr;
                                contact_point_pf = p2_min;
                                pen_axis = a1_i + 1;
                                pf_elems = (Face(a1_i), Vertex(p2_i as u8));
                            }
                        }
                    }
                }
                // f-p contacts
                let points_1 = CUBE_VERTICES.map(|v| v - pos_2_in_1);
                for p1_i in 0..4 {
                    let p1_min: Vector = axes_2_inv.map(|a| points_1[p1_i].dot(a)).into();
                    let p1_max: Vector = axes_2_inv.map(|a| points_1[7 - p1_i].dot(a)).into();
                    let ray = Ray {
                        origin: p1_min,
                        direction: p1_max - p1_min,
                        distance: 1.,
                    };
                    let (close, far) = ray.box_intersection_raw(&unit_bounds);

                    if close <= far && far >= 0. && close <= 1. {
                        let min_depth = far; // min point pen into far face
                        let max_depth = 1. - close; // max point pen into close face

                        // use smaller depth
                        if min_depth > max_depth {
                            let p2 = ray.calc_point(close);
                            let a2_i = CuboidElement::closest_face(p2);
                            let d_local = 1. - p1_max[a2_i as usize].abs(); // should be +ve i swear
                            let depth_sqr = d_local * d_local * model_2_sqr[a2_i as usize];

                            if depth_sqr > max_pen_pf_sqr {
                                max_pen_pf_sqr = depth_sqr;
                                contact_point_pf = CUBE_VERTICES[7 - p1_i];
                                pen_axis = a2_i + 4;
                                pf_elems = (Vertex(7 - p1_i as u8), Face(a2_i));
                                // use max index for p2
                            }
                        } else {
                            let p2 = ray.calc_point(far);
                            let a2_i = CuboidElement::closest_face(p2);
                            let d_local = 1. - p1_min[a2_i as usize].abs(); // should be +ve i swear
                            let depth_sqr = d_local * d_local * model_2_sqr[a2_i as usize];

                            if depth_sqr > max_pen_pf_sqr {
                                max_pen_pf_sqr = depth_sqr;
                                contact_point_pf = CUBE_VERTICES[p1_i];
                                pen_axis = a2_i + 4;
                                pf_elems = (Vertex(p1_i as u8), Face(a2_i));
                            }
                        }
                    }
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
                    let normal = axes[pen_axis as usize - 1];
                    // pen_axis.signum() as f32 * axes[pen_axis.unsigned_abs() as usize - 1];
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
                        0,
                    );
                    self.contact_resolver.add_contact(index, contact);
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
                        0,
                    );
                    self.contact_resolver.add_contact(index, contact);
                }
            }
        }

        self.add_cached_contacts(transforms);

        &mut self.contact_resolver
    }

    fn add_cached_contacts(&mut self, transform_sys: &mut TransformSystem) {
        let contacts = self.contact_resolver.get_contacts();
        let mut cached_contacts = Vec::with_capacity(contacts.len());
        for contact in contacts {
            let (rb_1, o_rb_2) = contact.get_rigidbodies();

            for (age, contact) in rb_1.write().unwrap().past_contacts.drain(..) {
                println!("[Uncaching contacts] age: {:?}", age);
                if let Some(res) = contact.into_contact(age, transform_sys) {
                    cached_contacts.push(res);
                }
            }
            if let Some(rb_2) = o_rb_2 {
                for (age, contact) in rb_2.write().unwrap().past_contacts.drain(..) {
                    println!("[Uncaching contacts] age: {:?}", age);
                    if let Some(res) = contact.into_contact(age, transform_sys) {
                        cached_contacts.push(res);
                    }
                }
            }
        }

        for (position, normal, penetration, contact_id, age) in cached_contacts {
            let (index, contact) = Contact::new(
                transform_sys,
                position,
                normal,
                penetration,
                contact_id,
                age,
            );
            self.contact_resolver.add_contact(index, contact);
        }
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

impl ContactIdPair {
    fn into_contact(
        self,
        age: u8,
        transform_sys: &mut TransformSystem,
    ) -> Option<(Vector, Vector, f32, Self, u8)> {
        let coll_1 = self.0.collider.upgrade()?;
        let coll_2 = self.1.collider.upgrade()?;

        let model_1 = transform_sys.get_global_model(&coll_1.transform).unwrap(); // impl func to try get model without mut
        let model_2 = transform_sys.get_global_model(&coll_2.transform).unwrap();

        let (position, normal, penetration) = match (&self.0.element, &self.1.element) {
            (Vertex(v), Face(f)) => {
                let p1_world = model_1 * CUBE_VERTICES[*v as usize].extend(1.0);
                let p1_2 = (model_2.invert().unwrap() * p1_world).truncate();
                let d1 = p1_2.map(|c| 1. - c.abs());

                if d1.x < CACHED_NEG_DEPTH || d1.y < CACHED_NEG_DEPTH || d1.z < CACHED_NEG_DEPTH {
                    return None;
                }

                let depth_world = d1[*f as usize] * model_2[*f as usize].magnitude();

                (
                    p1_world.truncate(),
                    model_2[*f as usize].truncate().normalize(),
                    depth_world,
                )
            }
            (Face(f), Vertex(v)) => {
                let p2_world = model_2 * CUBE_VERTICES[*v as usize].extend(1.0);
                let p2_1 = (model_1.invert().unwrap() * p2_world).truncate();
                let d2 = p2_1.map(|c| 1. - c.abs());

                if d2.x < CACHED_NEG_DEPTH || d2.y < CACHED_NEG_DEPTH || d2.z < CACHED_NEG_DEPTH {
                    return None;
                }

                let depth_world = d2[*f as usize] * model_1[*f as usize].magnitude();

                (
                    p2_world.truncate(),
                    model_1[*f as usize].truncate().normalize(),
                    depth_world,
                )
            }
            (Edge(e1), Edge(e2)) => {
                let (p1, a1_i) = CuboidElement::into_vertex_axis(*e1);
                let (p2, a2_i) = CuboidElement::into_vertex_axis(*e2);

                let p1 = (model_1 * p1.extend(1.)).truncate();
                let p2 = (model_2 * p2.extend(1.)).truncate();
                let a1 = model_1[a1_i].truncate();
                let a2 = model_2[a2_i].truncate();

                let normal = a1.cross(a2).normalize();

                let p2_p1 = p2 - p1;
                // let depth = p2_p1.dot(normal).abs();

                let n2 = a2.cross(normal);
                let c1 = p1 + (p2_p1.dot(n2) / a1.dot(n2)) * a1;

                let n1 = a1.cross(normal);
                let c2 = p2 - (p2_p1.dot(n1) / a2.dot(n1)) * a2;

                let depth = c1.distance(model_1.w.truncate()) - c2.distance(model_1.w.truncate());

                // its an intercept if c1 is closer to the centre of 2 than the centre of 1
                if depth < CACHED_NEG_DEPTH {
                    return None;
                } else {
                    (c1, normal, depth)
                }
            }
            _ => {
                panic!(
                    "Invalid contact id pair: {:?} {:?}",
                    self.0.element, self.1.element
                )
            }
        };

        Some((position, normal, penetration, self, age)) // todo: make contact create info struct
    }
}

impl CuboidElement {
    fn from_vertex_axis(vertex: u8, axis: u8) -> Self {
        let axis_mask = 1 << axis;
        let axis_flags = axis << 3;
        Edge(axis_flags | vertex | axis_mask)
    }

    fn into_vertex_axis(edge: u8) -> (Vector, usize) {
        let vertex = edge & 0b111; // get 3 least sig bits
        let axis = edge >> 3; // get 2 most sig bits

        (CUBE_VERTICES[vertex as usize], axis as usize)
    }

    // fn get_edge_axis(edge: u8) -> u8 {
    //     edge >> 3
    // }

    fn closest_face(point: Vector) -> u8 {
        let mut max = point.x.abs();
        let mut axis = 0;

        if point.y.abs() > max {
            max = point.y.abs();
            axis = 1;
        }
        if point.z.abs() > max {
            axis = 2;
        }

        axis
    }

    fn closest_vertex(point: Vector) -> u8 {
        let mut result = 0;
        if point.x > 0. {
            result += 0b001;
        }
        if point.y > 0. {
            result += 0b010;
        }
        if point.z > 0. {
            result += 0b100;
        }
        result
    }
}

#[cfg(test)]
mod coll_tests {
    use super::CuboidElement;

    #[test]
    fn bit_manips() {
        let vertex = CuboidElement::closest_vertex((0.12, 1.2, -2.).into());
        let edge = CuboidElement::from_vertex_axis(1, 1);
        let (v, a) = CuboidElement::into_vertex_axis(0b01_010);

        println!("c_v: {:?}", vertex);
        println!("e: {:?}", edge);
        println!("v: {:?}, a: {:?}", v, a);
    }
}
