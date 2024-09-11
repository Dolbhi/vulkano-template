mod bounds_tree;

use std::{
    fmt::Debug,
    sync::{Arc, Mutex},
};

use cgmath::{InnerSpace, Rotation, Vector3, Vector4, Zero};

use bounds_tree::{BoundsTree, TreeIter};

use crate::game_objects::transform::{self, TransformID, TransformSystem, TransformView};

use super::Vector;

pub use self::bounds_tree::Leaf;
pub type ColliderRef = Arc<Mutex<Leaf>>;

#[derive(Clone, Copy, Debug)]
pub struct BoundingBox {
    pub max: Vector,
    pub min: Vector,
}
pub struct CuboidCollider {
    transform: TransformID,
    bounding_box: BoundingBox,
}
pub struct ColliderSystem {
    bounds_tree: BoundsTree,
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

    fn check_overlap(&self, other: Self) -> bool {
        let d1 = other.min - self.min;
        let d2 = other.max - self.max;

        d1.x < 0.0 && d1.y < 0.0 && d1.z < 0.0 && d2.x < 0.0 && d2.y < 0.0 && d2.z < 0.0
    }

    fn translate(&mut self, translation: Vector) {
        self.max += translation;
        self.min += translation;
    }

    fn join(self, rhs: Self) -> Self {
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

    fn volume(&self) -> f32 {
        let extends = self.max - self.min;
        extends.x * extends.y * extends.z
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

const CUBE_BOUNDING: [Vector; 3] = [
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
];

impl CuboidCollider {
    pub fn new(transforms: &mut TransformSystem, transform: TransformID) -> Self {
        let mut collider = CuboidCollider {
            transform,
            bounding_box: BoundingBox::default(),
        };
        collider.update_bounding(transforms);
        collider
    }

    fn update_bounding(&mut self, transforms: &mut TransformSystem) {
        let global_model = transforms.get_global_model(&self.transform).unwrap();
        let view = transforms
            .get_transform(&self.transform)
            .unwrap()
            .get_local_transform();

        let pos = global_model * Vector4::new(1.0, 0.0, 0.0, 1.0);

        self.bounding_box.min = pos.truncate() / pos.w;
        self.bounding_box.max = self.bounding_box.min + view.scale;

        // let vertices = CUBE_BOUNDING.map(|v| view.rotation.rotate_vector(v));

        // self.bounding_box = BoundingBox::from_vertices(&vertices);
        // self.bounding_box.translate(*view.translation);
    }
}
impl Debug for CuboidCollider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Collider({})", self.transform.id()))
    }
}

impl ColliderSystem {
    pub fn new() -> Self {
        Self {
            bounds_tree: BoundsTree::new(),
        }
    }

    pub fn tree_depth(&self) -> u32 {
        self.bounds_tree.depth()
    }

    // removed update and reinsert given collider
    pub fn update(&mut self, target: &ColliderRef, transforms: &mut TransformSystem) {
        // println!("[Updating bounds]");
        self.bounds_tree.remove(target);
        // println!("\t[Removed]");
        let bounds = {
            let mut lock = target.lock().unwrap();
            lock.collider.update_bounding(transforms);
            lock.collider.bounding_box
        };
        // println!("\t[Updated]");
        self.bounds_tree.insert_leaf(target, bounds);
        // println!("\t[Inserted]");
    }

    /// adds collider to bounds tree, returns a reference to its leaf node
    pub fn add(&mut self, collider: CuboidCollider) -> ColliderRef {
        self.bounds_tree.insert_new(collider)
    }
    pub fn remove(&mut self, target: &Arc<Mutex<Leaf>>) {
        self.bounds_tree.remove(target);
    }

    pub fn bounds_iter(&self) -> TreeIter {
        self.bounds_tree.iter()
    }
}
