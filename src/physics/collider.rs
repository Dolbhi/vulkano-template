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
    use crate::{game_objects::transform::TransformSystem, physics::collider::remove};

    use super::{BoundingBox, ColliderSystem, Node};

    fn validate_tree(child: &(BoundingBox, Node)) -> Result<i32, String> {
        match &child.1 {
            Node::None => Ok(0),
            Node::Leaf(_) => Ok(0),
            Node::Branch(branch) => {
                let branch_lock = branch.lock().unwrap();

                if child.0 != branch_lock.bounds() {
                    return Err("Bounds set incorrectly".to_string());
                }

                let left_depth = validate_tree(&branch_lock.left)?;
                let right_depth = validate_tree(&branch_lock.right)?;

                if branch_lock.balance != right_depth - left_depth {
                    return Err("Balance set incorrectly".to_string());
                }

                Ok(right_depth.max(left_depth + 1))
            }
        }
    }

    #[test]
    fn insert_test() {
        let mut trans = TransformSystem::new();
        let mut colls = ColliderSystem::new();

        let crap_box = super::BoundingBox {
            max: (1.0, 1.0, 1.0).into(),
            min: (0.0, 0.0, 0.0).into(),
        };
        let box_2 = super::BoundingBox {
            max: (2.0, 2.0, 2.0).into(),
            min: (1.0, 1.0, 1.0).into(),
        };

        colls.insert(super::CuboidCollider {
            transform: trans.next().unwrap(),
            bounding_box: crap_box,
        });
        colls.insert(super::CuboidCollider {
            transform: trans.next().unwrap(),
            bounding_box: box_2,
        });
        colls.insert(super::CuboidCollider {
            transform: trans.next().unwrap(),
            bounding_box: box_2,
        });

        let validate_start = (crap_box.join(box_2), colls.collider_root);
        let validation = validate_tree(&validate_start);
        assert_eq!(validation, Ok(2), "Tree: \n{:#?}", validate_start.1);
        assert_eq!("ALL", "GOOD", "Tree: \n{:#?}", validate_start.1);
    }
    #[test]
    fn remove_test() {
        let mut trans = TransformSystem::new();
        let mut colls = ColliderSystem::new();

        let crap_box = super::BoundingBox {
            max: (1.0, 1.0, 1.0).into(),
            min: (0.0, 0.0, 0.0).into(),
        };
        let box_2 = super::BoundingBox {
            max: (2.0, 2.0, 2.0).into(),
            min: (1.0, 1.0, 1.0).into(),
        };

        colls.insert(super::CuboidCollider {
            transform: trans.next().unwrap(),
            bounding_box: crap_box,
        });
        let to_remove = colls.insert(super::CuboidCollider {
            transform: trans.next().unwrap(),
            bounding_box: box_2,
        });
        colls.insert(super::CuboidCollider {
            transform: trans.next().unwrap(),
            bounding_box: box_2,
        });

        remove(&to_remove);

        let validate_start = (crap_box.join(box_2), colls.collider_root);
        let validation = validate_tree(&validate_start);
        assert_eq!(validation, Ok(1), "Tree: \n{:#?}", validate_start.1);
        assert_eq!("ALL", "GOOD", "Tree: \n{:#?}", validate_start.1);
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
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
impl Debug for CuboidCollider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Collider({})", self.transform.id()))
    }
}

pub struct ColliderSystem {
    collider_root: Node,
}
impl ColliderSystem {
    pub fn new() -> Self {
        Self {
            collider_root: Node::None,
        }
    }

    pub fn insert(&mut self, collider: CuboidCollider) -> Arc<Mutex<Leaf>> {
        let (new_root, new_leaf) = match std::mem::take(&mut self.collider_root) {
            Node::None => {
                let new_leaf = Arc::new(Mutex::new(Leaf {
                    parent: Weak::new(),
                    // right_child: true,
                    collider,
                }));
                (Node::Leaf(new_leaf.clone()), new_leaf)
            }
            Node::Leaf(leaf) => {
                let (new_branch, new_leaf) = insert_leaf(leaf, collider); //right: bool
                (Node::Branch(new_branch), new_leaf)
            }
            Node::Branch(branch) => {
                let new_leaf = insert_branch(&mut branch.lock().unwrap(), collider);
                (Node::Branch(branch), new_leaf)
            }
        };
        self.collider_root = new_root;
        new_leaf
    }
    pub fn remove(&mut self, leaf: &Arc<Mutex<Leaf>>) {
        if let Some(new_root) = remove(leaf) {
            self.collider_root = new_root;
        }
    }
}

#[derive(Default, Clone)]
enum Node {
    #[default]
    None,
    Leaf(Arc<Mutex<Leaf>>),
    Branch(Arc<Mutex<Branch>>),
}
impl Node {
    fn ptr_eq(&self, other: &Node) -> bool {
        match (self, other) {
            (Node::None, Node::None) => true,
            (Node::Leaf(a), Node::Leaf(b)) => Arc::ptr_eq(a, b),
            (Node::Branch(a), Node::Branch(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
    fn leaf_eq(&self, other: &Arc<Mutex<Leaf>>) -> bool {
        if let Node::Leaf(this) = self {
            Arc::ptr_eq(this, other)
        } else {
            false
        }
    }
    fn branch_eq(&self, other: &Arc<Mutex<Branch>>) -> bool {
        if let Node::Branch(this) = self {
            Arc::ptr_eq(this, other)
        } else {
            false
        }
    }
}
impl Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "None"),
            Self::Leaf(arg0) => arg0.lock().unwrap().fmt(f),
            Self::Branch(arg0) => arg0.lock().unwrap().fmt(f),
        }
    }
}

struct Leaf {
    parent: Weak<Mutex<Branch>>,
    // right_child: bool,
    collider: CuboidCollider,
}
impl Debug for Leaf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Leaf")
            // .field("right_child", &self.right_child)
            .field("collider", &self.collider)
            .finish()
    }
}

struct Branch {
    parent: Weak<Mutex<Branch>>,
    // right_child: bool,
    left: (BoundingBox, Node),
    right: (BoundingBox, Node),
    balance: i32,
}
impl Branch {
    fn bounds(&self) -> BoundingBox {
        self.left.0.join(self.right.0)
    }

    /// update bounds and balance of parents of removed child
    fn update_removal(
        parent: &Arc<Mutex<Branch>>,
        bounds: BoundingBox,
        removed_origin: &Arc<Mutex<Branch>>,
    ) {
        let mut lock = parent.lock().unwrap();

        // update child and self
        if lock.right.1.branch_eq(removed_origin) {
            lock.right.0 = bounds;
            lock.balance -= 1;
        } else {
            lock.left.0 = bounds;
            lock.balance += 1;
        }

        // propergate to parent if it exists
        if let Some(next_parent) = lock.parent.upgrade() {
            Self::update_removal(&next_parent, lock.bounds(), parent);
        }
    }
    /// update bounds and balance of parents of removed child
    fn update_removal_bool(parent: &Arc<Mutex<Branch>>, bounds: BoundingBox, removed_origin: bool) {
        let mut lock = parent.lock().unwrap();

        // update child and self
        if removed_origin {
            lock.right.0 = bounds;
            lock.balance -= 1;
        } else {
            lock.left.0 = bounds;
            lock.balance += 1;
        }

        // propergate to parent if it exists
        if let Some(next_parent) = lock.parent.upgrade() {
            Self::update_removal(&next_parent, lock.bounds(), parent);
        }
    }

    // fn mut_child(&mut self, right_child: bool) -> &mut (BoundingBox, Node) {
    //     if right_child {
    //         &mut self.right
    //     } else {
    //         &mut self.left
    //     }
    // }
}
impl Debug for Branch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Branch")
            // .field("right_child", &self.right_child)
            .field("balance", &self.balance)
            .field("left", &self.left.1)
            .field("right", &self.right.1)
            .finish()
    }
}

fn insert_leaf(
    leaf: Arc<Mutex<Leaf>>,
    collider: CuboidCollider,
    // right_child: bool,
) -> (Arc<Mutex<Branch>>, Arc<Mutex<Leaf>>) {
    let mut new_leaf: Option<Arc<Mutex<Leaf>>> = None;

    let new_branch = Arc::new_cyclic(|new_parent| {
        let mut leaf_lock = leaf.lock().unwrap();
        // leaf_lock.right_child = true;
        let parent = std::mem::replace(&mut leaf_lock.parent, new_parent.clone());
        let bounding_box = leaf_lock.collider.bounding_box;
        drop(leaf_lock);

        let new_bounds = collider.bounding_box;
        let leaf_node = Arc::new(Mutex::new(Leaf {
            parent: new_parent.clone(),
            // right_child: false,
            collider,
        }));
        new_leaf = Some(leaf_node.clone());

        Mutex::new(Branch {
            parent,
            // right_child,
            left: (new_bounds, Node::Leaf(leaf_node)),
            right: (bounding_box, Node::Leaf(leaf)),
            balance: 0,
        })
    });

    (new_branch, new_leaf.unwrap())
}

fn insert_branch(branch: &mut Branch, collider: CuboidCollider) -> Arc<Mutex<Leaf>> {
    let left_bounding = branch.left.0.join(collider.bounding_box);
    let right_bounding = branch.right.0.join(collider.bounding_box);

    if left_bounding.volume() - branch.left.0.volume()
        < right_bounding.volume() - branch.right.0.volume()
    {
        branch.left.0 = left_bounding;
        branch.balance -= 1;
        match std::mem::take(&mut branch.left.1) {
            Node::Leaf(leaf) => {
                let (new_branch, new_leaf) = insert_leaf(leaf, collider); //right: false
                branch.left.1 = Node::Branch(new_branch);
                new_leaf
            }
            Node::Branch(branch) => insert_branch(&mut branch.lock().unwrap(), collider),
            Node::None => {
                panic!("Branch has empty child")
            }
        }
    } else {
        branch.right.0 = right_bounding;
        branch.balance += 1;
        match std::mem::take(&mut branch.right.1) {
            Node::Leaf(leaf) => {
                let (new_branch, new_leaf) = insert_leaf(leaf, collider); //right: true
                branch.right.1 = Node::Branch(new_branch);
                new_leaf
            }
            Node::Branch(branch) => insert_branch(&mut branch.lock().unwrap(), collider),
            Node::None => {
                panic!("Branch has empty child")
            }
        }
    }
}

/// If the root branch gets replaced by removal, returns the new root
fn remove(leaf: &Arc<Mutex<Leaf>>) -> Option<Node> {
    let leaf_lock = leaf.lock().unwrap();
    if let Some(parent) = leaf_lock.parent.upgrade() {
        let parent_lock = parent.lock().unwrap();
        // get sibling of removed
        let (bounds, sibling) = if parent_lock.right.1.leaf_eq(leaf) {
            parent_lock.right.clone()
        } else {
            parent_lock.left.clone()
        };

        if let Some(grandparent) = parent_lock.parent.upgrade() {
            let mut grand_lock = grandparent.lock().unwrap();

            // find parent to replace
            let right = grand_lock.right.1.branch_eq(&parent);
            if right {
                grand_lock.right = (bounds, sibling);
            } else {
                grand_lock.left = (bounds, sibling);
            }

            drop(grand_lock);

            Branch::update_removal_bool(&grandparent, bounds, right);

            None
        } else {
            Some(sibling)
        }
    } else {
        Some(Node::None)
    }
}
