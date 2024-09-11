use std::{
    fmt::Debug,
    ops::Not,
    sync::{Arc, Mutex, Weak},
};

use super::{BoundingBox, CuboidCollider};

fn arcmutex<T>(thing: T) -> Arc<Mutex<T>> {
    Arc::new(Mutex::new(thing))
}

pub struct BoundsTree {
    /// Depth is invalid LOL
    root: Option<Link>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ChildSide {
    Left,
    Right,
}
use ChildSide::*;

pub struct Leaf {
    parent: Weak<Mutex<Branch>>,
    right_child: ChildSide,
    pub collider: CuboidCollider,
}
struct Branch {
    parent: Weak<Mutex<Branch>>,
    right_child: ChildSide,
    children: [Link; 2],
}
#[derive(Clone)]
struct Link {
    node: Arc<Mutex<dyn Node>>,
    bounds: BoundingBox,
    depth: u32,
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

/// Depth first iterator for `BoundsTree`
pub struct TreeIter {
    current: Vec<Link>,
    next: Vec<Link>,
}

impl BoundsTree {
    pub fn new() -> Self {
        Self { root: None }
    }

    pub fn depth(&self) -> u32 {
        if let Some(ref root) = self.root {
            root.depth
        } else {
            0
        }
    }

    pub fn insert(&mut self, collider: CuboidCollider) -> Arc<Mutex<Leaf>> {
        let bounds = collider.bounding_box;
        let new_leaf_node = arcmutex(Leaf {
            parent: Weak::new(),
            right_child: Right,
            collider,
        });
        let new_leaf = Link::new(new_leaf_node.clone(), bounds, 0);
        match &mut self.root {
            None => {
                // empty root, make new leaf
                self.root = Some(new_leaf);
            }
            Some(node) => {
                // existing root, use insert func
                node.bounds = node.bounds.join(new_leaf.bounds);
                node.insert(new_leaf);
            }
        }
        new_leaf_node
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

    pub fn merge(&mut self, other: BoundsTree) {
        if let Some(other_root) = other.root {
            match &mut self.root {
                None => {
                    // empty root, make new leaf
                    self.root = Some(other_root);
                }
                Some(node) => {
                    // existing root, use insert func
                    node.bounds = node.bounds.join(other_root.bounds);
                    node.insert(other_root);
                }
            }
        }
    }

    pub fn iter(&self) -> TreeIter {
        if let Some(ref root) = self.root {
            TreeIter {
                current: vec![root.clone()],
                next: vec![],
            }
        } else {
            TreeIter {
                current: vec![],
                next: vec![],
            }
        }
    }
}
impl IntoIterator for &BoundsTree {
    type IntoIter = TreeIter;
    type Item = (BoundingBox, u32);

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl Iterator for TreeIter {
    type Item = (BoundingBox, u32);

    fn next(&mut self) -> Option<Self::Item> {
        match self.current.pop() {
            Some(link) => {
                if let Some(branch) = link.node.lock().unwrap().try_into_branch() {
                    self.next.push(branch.children[0].clone());
                    self.next.push(branch.children[1].clone());
                }
                Some((link.bounds, link.depth))
            }
            None => {
                // swap current and next
                std::mem::swap(&mut self.current, &mut self.next);

                // try pop current again
                if let Some(link) = self.current.pop() {
                    if let Some(branch) = link.node.lock().unwrap().try_into_branch() {
                        self.next.push(branch.children[0].clone());
                        self.next.push(branch.children[1].clone());
                    }
                    Some((link.bounds, link.depth))
                } else {
                    // both vecs empty
                    None
                }
            }
        }
    }
}

impl Link {
    fn new(node: Arc<Mutex<dyn Node>>, bounds: BoundingBox, depth: u32) -> Self {
        Self {
            node,
            bounds,
            depth,
        }
    }
    // fn from_mutex_lock(lock: &dyn Node, node: &Arc<Mutex<dyn Node>>) -> Self {
    //     Self {
    //         node: node.clone(),
    //         bounds: lock.bounds(),
    //         depth: lock.depth(),
    //     }
    // }

    /// Inserted child must be right
    fn insert(&mut self, new_leaf: Link) {
        let mut lock = self.node.lock().unwrap();

        // branch or leaf?
        if let Some(branch) = lock.try_into_branch_mut() {
            // get potential bounds and volume change
            let new_bounds: Vec<(BoundingBox, f32)> = branch
                .children
                .iter()
                .map(|Link { bounds, .. }| {
                    let new = bounds.join(new_leaf.bounds);
                    (new, new.volume() - bounds.volume())
                })
                .collect();

            // pick child and update bounds
            let next_child = (if new_bounds[0].1 < new_bounds[1].1 {
                Left
            } else {
                Right
            }) as usize;
            branch.children[next_child].bounds = new_bounds[next_child].0;

            branch.children[next_child].insert(new_leaf);

            // rebalance if needed
            branch.rebalance();
            self.depth = branch.depth();
        } else {
            // convert leaf to branch
            let mut new_depth = 0;
            let branch = Arc::new_cyclic(|new_branch| {
                let parent = lock.parent().clone();
                let right_child = lock.right_child();

                lock.set_parent(new_branch.clone(), Left);
                let mut new_left = self.clone();
                new_left.bounds = lock.bounds();

                {
                    new_leaf
                        .node
                        .lock()
                        .unwrap()
                        .set_parent(new_branch.clone(), Right);
                }
                let new_right = new_leaf;

                new_depth = new_left.depth.max(new_right.depth) + 1;
                Mutex::new(Branch {
                    parent,
                    right_child,
                    children: [new_left, new_right],
                })
            });

            drop(lock);
            self.depth = new_depth;
            self.node = branch;
        }
    }
}
impl Debug for Link {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "depth: {}, node: {:#?}",
            self.depth,
            self.node.lock().unwrap()
        ))
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
impl From<usize> for ChildSide {
    fn from(value: usize) -> Self {
        if value == 0 {
            Left
        } else {
            Right
        }
    }
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

impl Branch {
    /// If self has no parent, return the replacement instead
    fn delete_child(&mut self, right_child: ChildSide) -> Option<Link> {
        let replacement = self.children[1 - right_child as usize].clone();

        if let Some(parent) = self.parent.upgrade() {
            {
                let mut lock = replacement.node.lock().unwrap();
                lock.set_parent(Arc::downgrade(&parent), self.right_child)
            }

            let mut parent_lock = parent.lock().unwrap();
            // replace self in parent
            parent_lock.children[self.right_child as usize] = replacement;

            parent_lock.rebalance();

            if let Some(grandparent) = parent_lock.parent.upgrade() {
                grandparent.lock().unwrap().update_removed(
                    parent_lock.right_child,
                    parent_lock.bounds(),
                    parent_lock.depth(),
                );
            }
            None
        } else {
            // self is root, make replacement the new root
            {
                let mut lock = replacement.node.lock().unwrap();
                lock.set_parent(Weak::new(), self.right_child)
            }
            Some(replacement)
        }
    }
    fn update_removed(&mut self, right_child: ChildSide, bounds: BoundingBox, depth: u32) {
        self.children[right_child as usize].bounds = bounds;
        self.children[right_child as usize].depth = depth;

        self.rebalance();

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

    /// Rebalance tree if needed
    fn rebalance(&mut self) {
        let unbalanced_child = match self.children[1].depth as i32 - self.children[0].depth as i32 {
            i if i <= -2 => Some(0),
            i if i >= 2 => Some(1),
            _ => None,
        };

        // rebalance if needed
        if let Some(next_child) = unbalanced_child {
            let bigger_lock = self.children[next_child].node.lock().unwrap();
            let bigger_child = bigger_lock.try_into_branch().unwrap();
            let bigger_index = if bigger_child.children[0].depth > bigger_child.children[1].depth {
                0
            } else {
                1
            };
            let weak_self = bigger_child.parent.clone();

            let bigger_grand = bigger_child.children[bigger_index].clone();
            let smaller_grand = bigger_child.children[1 - bigger_index].clone();

            drop(bigger_lock);

            bigger_grand
                .node
                .lock()
                .unwrap()
                .set_parent(weak_self, next_child.into());
            self.children[next_child] = bigger_grand;

            self.children[1 - next_child].bounds = self.children[1 - next_child]
                .bounds
                .join(smaller_grand.bounds);
            self.children[1 - next_child].insert(smaller_grand);
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
            .field("left", &self.children[0])
            .field("right", &self.children[1])
            .finish()
    }
}

#[cfg(test)]
mod tree_tests {
    use std::sync::{Arc, Mutex};

    use crate::{
        game_objects::transform::TransformSystem,
        physics::collider::{bounds_tree::BoundsTree, CuboidCollider},
    };

    use super::{ChildSide, Link, Node};

    fn validate_tree(
        child: &Link,
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
            return Err(
                format!("Incorrect depth: {:?} vs {:?}", child.depth, lock.depth()).to_string(),
            );
        }

        if let Some(branch) = lock.try_into_branch() {
            if (branch.children[0].depth).abs_diff(branch.children[1].depth) > 1 {
                return Err("Unbalanced tree".to_string());
            }

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

    fn assert_valid_tree(root: &Link) {
        let mut root = root.clone();
        {
            let lock = root.node.lock().unwrap();
            root.depth = lock.depth(); // manually validate root depth
        }
        let validation = validate_tree(&root, None, ChildSide::Right);

        assert!(
            validation.is_ok(),
            "Err: {:?}, \nTree: {:#?}",
            validation,
            root.node.lock().unwrap()
        );
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

        assert_valid_tree(&tree.root.unwrap());
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

        assert_valid_tree(&tree.root.unwrap());
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

        assert_valid_tree(&tree.root.unwrap())
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

        assert_valid_tree(&tree.root.unwrap())
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
        assert_valid_tree(&tree.root.unwrap());
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
    #[test]
    fn merge_test() {
        let mut trans = TransformSystem::new();
        let mut tree1 = BoundsTree::new();
        let mut tree2 = BoundsTree::new();

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

        for bounding_box in [crap_box, box_2, box_5, box_6, box_2, box_4, box_6] {
            tree1.insert(CuboidCollider {
                transform: trans.next().unwrap(),
                bounding_box,
            });
        }
        for bounding_box in [box_5, box_2, box_3] {
            tree2.insert(CuboidCollider {
                transform: trans.next().unwrap(),
                bounding_box,
            });
        }
        let uwu = tree2.insert(CuboidCollider {
            transform: trans.next().unwrap(),
            bounding_box: crap_box,
        });

        tree1.merge(tree2);

        tree1.remove(uwu);

        assert_valid_tree(&tree1.root.unwrap());
    }
    #[test]
    fn removal_balance() {}
}
