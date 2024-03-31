use std::{
    fmt::Debug,
    sync::{Arc, Mutex, Weak},
};

// use cgmath::Matrix4;

use cgmath::Rotation;

use crate::game_objects::transform::{TransformID, TransformSystem, TransformView};

use super::Vector;

#[cfg(test)]
mod coll_tests {
    use crate::game_objects::transform::TransformSystem;

    use super::ColliderSystem;

    #[test]
    fn single_collider() {
        let mut trans = TransformSystem::new();
        let mut colls = ColliderSystem {
            collider_root: super::Node::None,
        };

        colls.insert(super::CuboidCollider {
            transform: trans.next().unwrap(),
            bounding_box: super::BoundingBox {
                max: (1.0, 1.0, 1.0).into(),
                min: (0.0, 0.0, 0.0).into(),
            },
        });

        println!("{:?}", colls.collider_root);
    }
}

#[derive(Clone, Copy, Debug)]
struct BoundingBox {
    pub max: Vector,
    pub min: Vector,
}
impl BoundingBox {
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

#[derive(Debug)]
struct CuboidCollider {
    transform: TransformID,
    bounding_box: BoundingBox,
}
impl CuboidCollider {
    fn update_bounding(&mut self, view: TransformView) {
        let vertices = CUBE_BOUNDING
            .clone()
            .map(|v| view.rotation.rotate_vector(v));

        self.bounding_box = BoundingBox::from_vertices(&vertices);
        self.bounding_box.translate(*view.translation);
    }
}

pub struct ColliderSystem {
    collider_root: Node,
}
impl ColliderSystem {
    pub fn insert(&mut self, collider: CuboidCollider) {
        self.collider_root = match std::mem::take(&mut self.collider_root) {
            Node::None => Node::Leaf(Arc::new(Mutex::new(Leaf {
                parent: Weak::new(),
                right_child: true,
                collider,
            }))),
            Node::Leaf(leaf) => Node::Branch(insert_leaf(leaf, collider, true)),
            Node::Branch(branch) => {
                insert_branch(&mut branch.lock().unwrap(), collider);
                Node::Branch(branch)
            }
        };
    }
    pub fn remove(&mut self, leaf: &Arc<Mutex<Leaf>>) {
        if let Some(new_root) = remove(leaf) {
            self.collider_root = new_root;
        }
    }
}

#[derive(Default, Debug)]
enum Node {
    #[default]
    None,
    Leaf(Arc<Mutex<Leaf>>),
    Branch(Arc<Mutex<Branch>>),
}

#[derive(Debug)]
struct Leaf {
    parent: Weak<Mutex<Branch>>,
    right_child: bool,
    collider: CuboidCollider,
}

#[derive(Debug)]
struct Branch {
    parent: Weak<Mutex<Branch>>,
    right_child: bool,
    left: (BoundingBox, Node),
    right: (BoundingBox, Node),
    balance: i32,
}
impl Branch {
    fn bounds(&self) -> BoundingBox {
        self.left.0.join(self.right.0)
    }

    /// update bounds and balance of parents of removed child
    fn update_removal(&mut self, bounds: BoundingBox, right_child: bool) {
        self.mut_child(right_child).0 = bounds;
        self.balance -= if right_child { 1 } else { -1 };

        if let Some(parent) = self.parent.upgrade() {
            parent
                .lock()
                .unwrap()
                .update_removal(self.bounds(), self.right_child);
        }
    }

    fn mut_child(&mut self, right_child: bool) -> &mut (BoundingBox, Node) {
        if right_child {
            &mut self.right
        } else {
            &mut self.left
        }
    }
}
// impl Debug for Branch {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         f.debug_struct("Branch").field("parent", &self.parent).field("right_child", &self.right_child).field("left", &self.left).field("right", &self.right).field("balance", &self.balance).finish()
//     }
// }

fn insert_leaf(
    leaf: Arc<Mutex<Leaf>>,
    collider: CuboidCollider,
    right_child: bool,
) -> Arc<Mutex<Branch>> {
    Arc::new_cyclic(|new_parent| {
        let mut leaf_lock = leaf.lock().unwrap();
        let parent = std::mem::replace(&mut leaf_lock.parent, new_parent.clone());
        let bounding_box = leaf_lock.collider.bounding_box;
        drop(leaf_lock);

        Mutex::new(Branch {
            parent,
            right_child,
            left: (bounding_box, Node::Leaf(leaf)),
            right: (
                collider.bounding_box,
                Node::Leaf(Arc::new(Mutex::new(Leaf {
                    parent: new_parent.clone(),
                    right_child: true,
                    collider,
                }))),
            ),
            balance: 0,
        })
    })
}

fn insert_branch(branch: &mut Branch, collider: CuboidCollider) {
    let left_bounding = branch.left.0.join(collider.bounding_box);
    let right_bounding = branch.right.0.join(collider.bounding_box);

    if left_bounding.volume() - branch.left.0.volume()
        < right_bounding.volume() - branch.right.0.volume()
    {
        branch.left.0 = left_bounding;
        branch.balance -= 1;
        match std::mem::take(&mut branch.left.1) {
            Node::Leaf(leaf) => {
                branch.left.1 = Node::Branch(insert_leaf(leaf, collider, false));
            }
            Node::Branch(branch) => insert_branch(&mut branch.lock().unwrap(), collider),
            Node::None => {}
        }
    } else {
        branch.right.0 = right_bounding;
        branch.balance += 1;
        match std::mem::take(&mut branch.right.1) {
            Node::Leaf(leaf) => {
                branch.right.1 = Node::Branch(insert_leaf(leaf, collider, true));
            }
            Node::Branch(branch) => insert_branch(&mut branch.lock().unwrap(), collider),
            Node::None => {}
        }
    }
}

/// If the root branch gets replaced by removal, returns the new root
fn remove(leaf: &Arc<Mutex<Leaf>>) -> Option<Node> {
    let leaf_lock = leaf.lock().unwrap();
    if let Some(parent) = leaf_lock.parent.upgrade() {
        let parent = Arc::into_inner(parent).unwrap().into_inner().unwrap();
        let sibling = if leaf_lock.right_child {
            parent.right
        } else {
            parent.left
        };

        if let Some(grandparent) = parent.parent.upgrade() {
            let mut grand_lock = grandparent.lock().unwrap();
            *grand_lock.mut_child(parent.right_child) = sibling;
            grand_lock.balance -= if parent.right_child { 1 } else { -1 };
            let bounds = grand_lock.right.0.join(grand_lock.left.0);
            grand_lock.update_removal(bounds, parent.right_child);

            None
        } else {
            Some(sibling.1)
        }
    } else {
        Some(Node::None)
    }
}
