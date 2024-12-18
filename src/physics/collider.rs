// mod bounds_tree;
mod bvh;

use std::fmt::Debug;

use cgmath::{InnerSpace, Zero};

use bvh::{DepthIter, LeafOutsideHierachy, BVH};

use crate::game_objects::transform::{TransformID, TransformSystem};

use super::Vector;

pub use self::bvh::LeafInHierachy;
// pub type ColliderRef = Arc<Mutex<Leaf>>;

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

const CUBE_BOUNDING: [Vector; 8] = [
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
    Vector {
        x: -1.0,
        y: -1.0,
        z: -1.0,
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
        // let view = transforms
        //     .get_transform(&self.transform)
        //     .unwrap()
        //     .get_local_transform();

        // let pos = global_model * Vector4::new(1.0, 0.0, 0.0, 1.0);

        // self.bounding_box.min = pos.truncate() / pos.w;
        // self.bounding_box.max = self.bounding_box.min + view.scale;

        let vertices = CUBE_BOUNDING.map(|v| {
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
}
