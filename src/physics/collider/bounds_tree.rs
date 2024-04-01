use std::{
    fmt::Debug,
    ops::Not,
    sync::{Arc, Mutex, Weak},
};

use super::{BoundingBox, CuboidCollider};

#[cfg(test)]
mod tree_tests {
    use std::sync::{Arc, Mutex};

    use crate::{
        game_objects::transform::TransformSystem, physics::collider::bounds_tree::BoundsTree,
    };

    use super::{BoundingBox, ChildSide, Node};

    fn validate_tree(
        child: &(BoundingBox, Arc<Mutex<dyn Node>>),
        side: ChildSide,
    ) -> Result<i32, String> {
        let lock = child.1.lock().unwrap();

        if lock.bounds() != child.0 {
            return Err("Invalid Bounds".to_string());
        }
        if lock.right_child() != side {
            return Err("Child side incorrect".to_string());
        }

        if let Some(branch) = lock.try_into_branch() {
            let left_depth = validate_tree(&branch.children[0], ChildSide::Left)?;
            let right_depth = validate_tree(&branch.children[1], ChildSide::Right)?;

            if right_depth - left_depth != branch.balance {
                return Err("Branch balance incorrect".to_string());
            }

            Ok(left_depth.max(right_depth) + 1)
        } else {
            Ok(0)
        }
    }

    #[test]
    fn insert_test() {
        let mut trans = TransformSystem::new();
        let mut tree = BoundsTree::new();

        let crap_box = super::BoundingBox {
            max: (1.0, 1.0, 1.0).into(),
            min: (0.0, 0.0, 0.0).into(),
        };
        let box_2 = super::BoundingBox {
            max: (2.0, 2.0, 2.0).into(),
            min: (1.0, 1.0, 1.0).into(),
        };

        tree.insert(super::CuboidCollider {
            transform: trans.next().unwrap(),
            bounding_box: crap_box,
        });
        tree.insert(super::CuboidCollider {
            transform: trans.next().unwrap(),
            bounding_box: box_2,
        });
        tree.insert(super::CuboidCollider {
            transform: trans.next().unwrap(),
            bounding_box: box_2,
        });

        let validate_start = (crap_box.join(box_2), tree.root.unwrap());
        let validation = validate_tree(&validate_start, ChildSide::Right);
        assert_eq!(validation, Ok(2), "Tree: \n{:#?}", validate_start.1);
        assert_eq!("ALL", "GOOD", "Tree: \n{:#?}", validate_start.1);
    }
    #[test]
    fn remove_test() {
        let mut trans = TransformSystem::new();
        let mut tree = BoundsTree::new();

        let crap_box = super::BoundingBox {
            max: (1.0, 1.0, 1.0).into(),
            min: (0.0, 0.0, 0.0).into(),
        };
        let box_2 = super::BoundingBox {
            max: (2.0, 2.0, 2.0).into(),
            min: (1.0, 1.0, 1.0).into(),
        };

        tree.insert(super::CuboidCollider {
            transform: trans.next().unwrap(),
            bounding_box: crap_box,
        });
        let to_remove = tree.insert(super::CuboidCollider {
            transform: trans.next().unwrap(),
            bounding_box: box_2,
        });
        tree.insert(super::CuboidCollider {
            transform: trans.next().unwrap(),
            bounding_box: box_2,
        });

        tree.remove(to_remove);

        let validate_start = (crap_box.join(box_2), tree.root.unwrap());
        let validation = validate_tree(&validate_start, ChildSide::Right);
        assert_eq!(validation, Ok(1), "Tree: \n{:#?}", validate_start.1);
        assert_eq!("ALL", "GOOD", "Tree: \n{:#?}", validate_start.1);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ChildSide {
    Left,
    Right,
}
use ChildSide::*;

// enum Root {
//     None,
//     One(Leaf),
//     Some(Arc<Mutex<dyn Node>>),
// }

fn arcmutex<T>(thing: T) -> Arc<Mutex<T>> {
    Arc::new(Mutex::new(thing))
}

pub struct BoundsTree {
    root: Option<Arc<Mutex<dyn Node>>>,
}
impl BoundsTree {
    pub fn new() -> Self {
        Self { root: None }
    }

    pub fn insert(&mut self, collider: CuboidCollider) -> Arc<Mutex<Leaf>> {
        match self.root.take() {
            None => {
                let new_leaf = arcmutex(Leaf {
                    parent: Weak::new(),
                    right_child: Right,
                    collider,
                });
                self.root = Some(new_leaf.clone());
                new_leaf
            }
            Some(node) => node.lock().unwrap().insert(collider),
        }
    }

    pub fn remove(&mut self, target: Arc<Mutex<Leaf>>) {
        Leaf::remove(target);
    }
}

trait Node: Debug {
    fn parent(&self) -> &Weak<Mutex<Branch>>;
    fn right_child(&self) -> ChildSide;
    fn set_parent(&mut self, parent: Weak<Mutex<Branch>>, right_child: ChildSide);
    fn bounds(&self) -> BoundingBox;
    fn is_leaf(&self) -> bool;
    fn try_into_branch(&self) -> Option<&Branch>;
    fn insert(&mut self, collider: CuboidCollider) -> Arc<Mutex<Leaf>>;
}

fn insert_to_leaf(
    tree_slot: &mut Arc<Mutex<dyn Node>>,
    collider: CuboidCollider,
) -> Arc<Mutex<Leaf>> {
    let mut fresh_leaf = None;

    let branch = Arc::new_cyclic(|branch| {
        let mut lock = tree_slot.lock().unwrap();
        let parent = lock.parent().clone();
        let right_child = lock.right_child();

        lock.set_parent(branch.clone(), Right);
        let new_right = (lock.bounds(), tree_slot.clone() as Arc<Mutex<dyn Node>>);

        let new_bounds = collider.bounding_box;
        let new_leaf = Arc::new(Mutex::new(Leaf {
            parent: branch.clone(),
            right_child: Left,
            collider,
        }));
        let new_left = (new_bounds, new_leaf.clone() as Arc<Mutex<dyn Node>>);

        fresh_leaf = Some(new_leaf);

        Mutex::new(Branch {
            parent,
            right_child,
            children: [new_left, new_right],
            balance: 0,
        })
    });

    *tree_slot = branch;

    fresh_leaf.unwrap()
}

pub struct Leaf {
    parent: Weak<Mutex<Branch>>,
    right_child: ChildSide,
    pub collider: CuboidCollider,
}
impl Leaf {
    fn remove(target: Arc<Mutex<Leaf>>) {
        let lock = target.lock().unwrap();
        lock.parent
            .upgrade()
            .unwrap()
            .lock()
            .unwrap()
            .delete_child(lock.right_child);
    }
}
impl Node for Leaf {
    fn parent(&self) -> &Weak<Mutex<Branch>> {
        &self.parent
    }
    fn right_child(&self) -> ChildSide {
        self.right_child
    }
    fn set_parent(&mut self, parent: Weak<Mutex<Branch>>, right_child: ChildSide) {
        self.parent = parent;
        self.right_child = right_child;
    }
    fn bounds(&self) -> BoundingBox {
        self.collider.bounding_box
    }
    fn try_into_branch(&self) -> Option<&Branch> {
        None
    }
    fn is_leaf(&self) -> bool {
        true
    }

    fn insert(&mut self, collider: CuboidCollider) -> Arc<Mutex<Leaf>> {
        panic!("Cannot insert to leaf")
    }
}
impl Debug for Leaf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Leaf")
            .field("parent", &self.parent)
            .field("right_child", &self.right_child)
            .field("collider", &self.collider)
            .finish()
    }
}

impl Not for ChildSide {
    type Output = Self;
    fn not(self) -> Self::Output {
        match self {
            Left => Right,
            Right => Left,
        }
    }
}
struct Branch {
    parent: Weak<Mutex<Branch>>,
    right_child: ChildSide,
    children: [(BoundingBox, Arc<Mutex<dyn Node>>); 2],
    balance: i32,
}
impl Branch {
    fn delete_child(&mut self, right_child: ChildSide) {
        let replacement = self.children[right_child as usize].clone();

        if let Some(parent) = self.parent.upgrade() {
            let mut parent_lock = parent.lock().unwrap();
            parent_lock.children[self.right_child as usize] = replacement;
            parent_lock.balance += 1 - 2 * right_child as i32;

            if let Some(grandparent) = parent_lock.parent.upgrade() {
                grandparent
                    .lock()
                    .unwrap()
                    .update_removed(parent_lock.right_child, parent_lock.bounds());
            }
        }
    }
    fn update_removed(&mut self, right_child: ChildSide, bounds: BoundingBox) {
        self.children[right_child as usize].0 = bounds;
        self.balance += 1 - 2 * right_child as i32;

        let right_child = self.right_child;
        let bounds = self.bounds();

        if let Some(parent) = self.parent.upgrade() {
            parent.lock().unwrap().update_removed(right_child, bounds);
        }
    }
}
impl Node for Branch {
    fn parent(&self) -> &Weak<Mutex<Branch>> {
        &self.parent
    }
    fn right_child(&self) -> ChildSide {
        self.right_child
    }
    fn set_parent(&mut self, parent: Weak<Mutex<Branch>>, right_child: ChildSide) {
        self.parent = parent;
        self.right_child = right_child;
    }
    fn bounds(&self) -> BoundingBox {
        self.children[0].0.join(self.children[1].0)
    }
    fn try_into_branch(&self) -> Option<&Branch> {
        Some(self)
    }
    fn is_leaf(&self) -> bool {
        false
    }

    fn insert(&mut self, collider: CuboidCollider) -> Arc<Mutex<Leaf>> {
        let new_bounds: Vec<(BoundingBox, f32)> = self
            .children
            .iter()
            .map(|(bounds, _)| {
                let new = bounds.join(collider.bounding_box);
                (new, new.volume() - bounds.volume())
            })
            .collect();

        let next = if new_bounds[0].1 < new_bounds[1].1 {
            self.balance -= 1;
            &mut self.children[0].1
        } else {
            self.balance += 1;
            &mut self.children[1].1
        };

        let mut lock = next.lock().unwrap();
        if lock.is_leaf() {
            drop(lock);
            insert_to_leaf(next, collider)
        } else {
            lock.insert(collider)
        }
    }
}
impl Debug for Branch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Branch")
            .field("right_child", &self.right_child)
            .field("balance", &self.balance)
            .field("children", &self.children)
            .finish()
    }
}
