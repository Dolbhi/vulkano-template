use std::{
    borrow::Borrow,
    fmt::Debug,
    ops::Not,
    sync::{Arc, Mutex, Weak},
};

use super::{BoundingBox, CuboidCollider};

#[cfg(test)]
mod tree_tests {
    use std::sync::{Arc, Mutex};

    use crate::{
        game_objects::transform::TransformSystem,
        physics::collider::{bounds_tree::BoundsTree, CuboidCollider},
    };

    use super::{BoundingBox, ChildSide, Node, NodeChild};

    fn validate_tree(
        child: &NodeChild,
        parent: Option<&Arc<Mutex<dyn Node>>>,
        side: ChildSide,
    ) -> Result<(), String> {
        let lock = child.node.lock().unwrap();

        if lock.bounds() != child.bounds {
            return Err(format!(
                "Incorrect Bounds: {:?} vs {:?}",
                lock.bounds(),
                child.bounds
            )
            .to_string());
        }
        if lock.right_child() != side {
            return Err("Incorrect child side".to_string());
        }
        if !(match (lock.parent().upgrade(), parent) {
            (Some(left), Some(right)) => Arc::ptr_eq(&(left as Arc<Mutex<dyn Node>>), right),
            (None, None) => true,
            (_, _) => false,
        }) {
            return Err("Incorrect parent".to_string());
        }
        if child.depth != lock.depth() {
            return Err("Incorrect branch depth".to_string());
        }

        if let Some(branch) = lock.try_into_branch() {
            validate_tree(
                &branch.children[0],
                Some(&child.node.clone()),
                ChildSide::Left,
            )?;
            validate_tree(
                &branch.children[1],
                Some(&child.node.clone()),
                ChildSide::Right,
            )?;
        }

        Ok(())
    }

    fn assert_depth(root: Arc<Mutex<dyn Node>>) {
        let node = root.lock().unwrap();

        let validate_start = NodeChild::from_mutex_lock(&*node, &root);
        drop(node);
        let validation = validate_tree(&validate_start, None, ChildSide::Right);

        let node = root.lock().unwrap();

        assert_eq!(validation, Ok(()), "Tree: \n{:#?}", node);
        // assert_eq!("ALL", "GOOD", "Tree: \n{:#?}", node);
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

        assert_depth(tree.root.unwrap());
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

        assert_depth(tree.root.unwrap());
    }
    #[test]
    fn big_tree() {
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
        let box_3 = super::BoundingBox {
            max: (5.0, 5.0, 6.0).into(),
            min: (4.0, 2.0, 4.0).into(),
        };
        let box_4 = super::BoundingBox {
            max: (1.0, 1.0, 1.0).into(),
            min: (-1.0, -1.0, -1.0).into(),
        };
        let box_5 = super::BoundingBox {
            max: (0.0, -1.0, 0.0).into(),
            min: (-5.0, -2.0, -5.0).into(),
        };
        let box_6 = super::BoundingBox {
            max: (5.0, -1.0, 20.0).into(),
            min: (2.0, -5.0, 5.0).into(),
        };

        for bounding_box in [
            crap_box, box_2, box_3, box_4, crap_box, box_5, box_6, box_2, box_4, box_6,
        ] {
            tree.insert(CuboidCollider {
                transform: trans.next().unwrap(),
                bounding_box,
            });
        }

        assert_depth(tree.root.unwrap())
    }
    #[test]
    fn big_remove() {
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
        let box_3 = super::BoundingBox {
            max: (5.0, 5.0, 6.0).into(),
            min: (4.0, 2.0, 4.0).into(),
        };
        let box_4 = super::BoundingBox {
            max: (1.0, 1.0, 1.0).into(),
            min: (-1.0, -1.0, -1.0).into(),
        };
        let box_5 = super::BoundingBox {
            max: (0.0, -1.0, 0.0).into(),
            min: (-5.0, -2.0, -5.0).into(),
        };
        let box_6 = super::BoundingBox {
            max: (5.0, -1.0, 20.0).into(),
            min: (2.0, -5.0, 5.0).into(),
        };

        let a = tree.insert(CuboidCollider {
            transform: trans.next().unwrap(),
            bounding_box: box_6,
        });

        for bounding_box in [
            crap_box, box_2, box_3, box_4, crap_box, box_5, box_6, box_2, box_4, box_6,
        ] {
            tree.insert(CuboidCollider {
                transform: trans.next().unwrap(),
                bounding_box,
            });
        }

        let b = tree.insert(CuboidCollider {
            transform: trans.next().unwrap(),
            bounding_box: box_2,
        });

        tree.remove(a);
        tree.remove(b);

        assert_depth(tree.root.unwrap())
    }
    #[test]
    fn remove_branch_root() {
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
        let remove = tree.insert(super::CuboidCollider {
            transform: trans.next().unwrap(),
            bounding_box: box_2,
        });

        tree.remove(remove);
        assert_depth(tree.root.unwrap());
    }
    #[test]
    fn remove_leaf_root() {
        let mut trans = TransformSystem::new();
        let mut tree = BoundsTree::new();

        let crap_box = super::BoundingBox {
            max: (1.0, 1.0, 1.0).into(),
            min: (0.0, 0.0, 0.0).into(),
        };
        let remove = tree.insert(super::CuboidCollider {
            transform: trans.next().unwrap(),
            bounding_box: crap_box,
        });

        tree.remove(remove);
        assert!(tree.root.is_none());
        // assert_depth(0, tree.root.unwrap());
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ChildSide {
    Left,
    Right,
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
use ChildSide::*;

#[derive(Clone)]
struct NodeChild {
    node: Arc<Mutex<dyn Node>>,
    bounds: BoundingBox,
    depth: u32,
}
impl NodeChild {
    fn new(node: Arc<Mutex<dyn Node>>, bounds: BoundingBox, depth: u32) -> Self {
        Self {
            node,
            bounds,
            depth,
        }
    }
    fn from_mutex_lock(lock: &dyn Node, node: &Arc<Mutex<dyn Node>>) -> Self {
        Self {
            node: node.clone(),
            bounds: lock.bounds(),
            depth: lock.depth(),
        }
    }
}

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
            Some(mut node) => {
                let (_, new_leaf) = insert(&mut node, collider);
                self.root = Some(node);
                new_leaf
            }
        }
    }

    pub fn remove(&mut self, target: Arc<Mutex<Leaf>>) {
        let lock = target.lock().unwrap();
        if let Some(parent) = lock.parent.upgrade() {
            if let Some(new_root) = parent.lock().unwrap().delete_child(lock.right_child) {
                // leaf as a new root
                self.root = Some(new_root);
            }
        } else {
            // single leaf on root, remove it
            self.root = None;
        }
    }
}

trait Node: Debug + Send + Sync {
    fn parent(&self) -> &Weak<Mutex<Branch>>;
    fn right_child(&self) -> ChildSide;
    fn set_parent(&mut self, parent: Weak<Mutex<Branch>>, right_child: ChildSide);
    fn bounds(&self) -> BoundingBox;
    fn depth(&self) -> u32;
    fn is_leaf(&self) -> bool;
    fn try_into_branch(&self) -> Option<&Branch>;
    fn try_into_branch_mut(&mut self) -> Option<&mut Branch>;
    // fn insert(&mut self, collider: CuboidCollider) -> Arc<Mutex<Leaf>>;
}

/// returns new depth of tree_slot and arc of new leaf
fn insert(
    tree_slot: &mut Arc<Mutex<dyn Node>>,
    collider: CuboidCollider,
) -> (u32, Arc<Mutex<Leaf>>) {
    let mut lock = tree_slot.lock().unwrap();

    // branch or leaf?
    if let Some(branch) = lock.try_into_branch_mut() {
        let new_bounds: Vec<(BoundingBox, f32)> = branch
            .children
            .iter()
            .map(|NodeChild { bounds, .. }| {
                let new = bounds.join(collider.bounding_box);
                (new, new.volume() - bounds.volume())
            })
            .collect();

        // pick child and update balance and bounds
        let next = if new_bounds[0].1 < new_bounds[1].1 {
            branch.children[0].bounds = new_bounds[0].0;
            &mut branch.children[0]
        } else {
            branch.children[1].bounds = new_bounds[1].0;
            &mut branch.children[1]
        };

        let mut result = insert(&mut next.node, collider);
        next.depth = result.0;
        result.0 = branch.depth();
        result
    } else {
        let mut fresh_leaf = None;

        let branch = Arc::new_cyclic(|branch| {
            let parent = lock.parent().clone();
            let right_child = lock.right_child();

            lock.set_parent(branch.clone(), Right);
            let new_right = NodeChild::from_mutex_lock(&*lock, &tree_slot);

            let new_bounds = collider.bounding_box;
            let new_leaf = Arc::new(Mutex::new(Leaf {
                parent: branch.clone(),
                right_child: Left,
                collider,
            }));
            let new_left = NodeChild::new(new_leaf.clone() as Arc<Mutex<dyn Node>>, new_bounds, 0);

            fresh_leaf = Some(new_leaf);

            Mutex::new(Branch {
                parent,
                right_child,
                children: [new_left, new_right],
            })
        });

        drop(lock);
        *tree_slot = branch;

        (1, fresh_leaf.unwrap())
    }
}

pub struct Leaf {
    parent: Weak<Mutex<Branch>>,
    right_child: ChildSide,
    pub collider: CuboidCollider,
}
impl Leaf {}
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
    fn depth(&self) -> u32 {
        0
    }
    fn try_into_branch(&self) -> Option<&Branch> {
        None
    }
    fn try_into_branch_mut(&mut self) -> Option<&mut Branch> {
        None
    }
    fn is_leaf(&self) -> bool {
        true
    }
}
impl Debug for Leaf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Leaf")
            .field("right_child", &self.right_child)
            .field("depth", &self.depth())
            .field("collider", &self.collider)
            .finish()
    }
}

struct Branch {
    parent: Weak<Mutex<Branch>>,
    right_child: ChildSide,
    children: [NodeChild; 2],
}
impl Branch {
    /// If self has no parent, return the replacement instead
    fn delete_child(&mut self, right_child: ChildSide) -> Option<Arc<Mutex<dyn Node>>> {
        let replacement = self.children[1 - right_child as usize].clone();

        if let Some(parent) = self.parent.upgrade() {
            {
                let mut lock = replacement.node.lock().unwrap();
                lock.set_parent(Arc::downgrade(&parent), self.right_child)
            }

            let mut parent_lock = parent.lock().unwrap();
            parent_lock.children[self.right_child as usize] = replacement;
            // parent_lock.balance += 1 - 2 * self.right_child as i32;

            if let Some(grandparent) = parent_lock.parent.upgrade() {
                grandparent.lock().unwrap().update_removed(
                    parent_lock.right_child,
                    parent_lock.bounds(),
                    parent_lock.depth(),
                );
            }
            None
        } else {
            Some(replacement.node)
        }
    }
    fn update_removed(&mut self, right_child: ChildSide, bounds: BoundingBox, depth: u32) {
        self.children[right_child as usize].bounds = bounds;
        self.children[right_child as usize].depth = depth;

        let right_child = self.right_child;
        let bounds = self.bounds();
        let depth = self.depth();

        if let Some(parent) = self.parent.upgrade() {
            parent
                .lock()
                .unwrap()
                .update_removed(right_child, bounds, depth);
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
        self.children[0].bounds.join(self.children[1].bounds)
    }
    fn depth(&self) -> u32 {
        self.children[0].depth.max(self.children[1].depth) + 1
    }
    fn try_into_branch(&self) -> Option<&Branch> {
        Some(self)
    }
    fn try_into_branch_mut(&mut self) -> Option<&mut Branch> {
        Some(self)
    }
    fn is_leaf(&self) -> bool {
        false
    }
}
impl Debug for Branch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Branch")
            .field("right_child", &self.right_child)
            .field("depth", &self.depth())
            .field("left", &self.children[0].node.lock().unwrap())
            .field("right", &self.children[1].node.lock().unwrap())
            .finish()
    }
}
