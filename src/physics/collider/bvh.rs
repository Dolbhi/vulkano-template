use std::{fmt::Debug, mem::ManuallyDrop, ptr::NonNull, sync::Arc};

use super::{BoundingBox, CuboidCollider};

pub struct BVH {
    root: Option<NonNull<Node>>,
    /// number of leafs (excluding those outside hierachy)
    size: usize,
}

#[allow(dead_code)]
pub struct Node {
    parent: Option<NonNull<Node>>,
    right_child: bool,
    bounds: BoundingBox,
    depth: usize,
    content: NodeContent,
}

#[allow(dead_code)]
enum NodeContent {
    Branch(NonNull<Node>, NonNull<Node>),
    Leaf(Arc<CuboidCollider>),
}

/// unique external reference, return to bvh to remove the corresponding leaf node
/// the NonNull node should only be dereferenced when provided with a &mut BVH matching the second element
#[allow(dead_code)]
pub struct LeafInHierachy {
    leaf: NonNull<Node>,
    hierachy: *const BVH,
}
#[allow(dead_code)]
pub struct LeafOutsideHierachy {
    leaf: NonNull<Node>,
    hierachy: *const BVH,
}

#[allow(unused)]
impl BVH {
    pub fn new() -> Self {
        BVH {
            root: None,
            size: 0,
        }
    }

    pub fn register_collider(&self, collider: Arc<CuboidCollider>) -> LeafOutsideHierachy {
        LeafOutsideHierachy {
            leaf: Node::new(collider),
            hierachy: self,
        }
    }
    // pub fn deregister_collider(&self, leaf_ref: LeafReference<OutsideHierachy>) {
    //     // properly drop a leaf node
    //     unsafe {
    //         Box::from_raw(leaf_ref.leaf.as_ptr());
    //     }
    // }

    pub fn insert(
        &mut self,
        leaf_ref: LeafOutsideHierachy,
    ) -> Result<LeafInHierachy, LeafOutsideHierachy> {
        if leaf_ref.hierachy != self {
            // Leaf does not belong to this hierachy
            return Err(leaf_ref);
        }

        if let Some(mut root) = self.root {
            unsafe {
                let leaf_bounds = leaf_ref.leaf.as_ref().bounds;

                // find best closest leaf
                let mut current = root;
                let mut new_bounds = (*current.as_ptr()).bounds.join(leaf_bounds);

                while let NodeContent::Branch(mut left, mut right) = (*current.as_ptr()).content {
                    (*current.as_ptr()).bounds = new_bounds;

                    let left_raw: &mut Node = left.as_mut();
                    let right_raw = right.as_mut();

                    let new_left_bounds = left_raw.bounds.join(leaf_bounds);
                    let new_right_bounds = right_raw.bounds.join(leaf_bounds);

                    if new_left_bounds.volume() - left_raw.bounds.volume()
                        <= new_right_bounds.volume() - right_raw.bounds.volume()
                    {
                        current = left;
                        new_bounds = new_left_bounds;
                    } else {
                        current = right;
                        new_bounds = new_right_bounds;
                    }
                }

                // turn leaf to branch
                let parent = (*current.as_ptr()).parent;
                let right_child = (*current.as_ptr()).right_child;

                let mut new_branch = NonNull::new_unchecked(Box::into_raw(Box::new(Node {
                    parent,
                    right_child,
                    bounds: new_bounds,
                    depth: 1,
                    content: NodeContent::Branch(current, leaf_ref.leaf),
                })));
                (*current.as_ptr()).parent = Some(new_branch);
                (*current.as_ptr()).right_child = false;
                (*leaf_ref.leaf.as_ptr()).parent = Some(new_branch);
                (*leaf_ref.leaf.as_ptr()).right_child = true;

                // update parent's content
                if let Some(mut parent) = parent {
                    let parent_raw = parent.as_mut();
                    if let NodeContent::Branch(left, right) = parent_raw.content {
                        if right_child {
                            parent_raw.content = NodeContent::Branch(left, new_branch);
                        } else {
                            parent_raw.content = NodeContent::Branch(new_branch, right);
                        }
                    } else {
                        // REEEEEEE
                    }
                } else {
                    self.root = Some(new_branch);
                }

                // update parent depths
                let mut last_node = new_branch.as_mut();
                let mut depth_changed = true;
                // let mut bounds_changed = false;
                while let Some(mut parent) = last_node.parent {
                    let parent_raw = parent.as_mut();

                    if let NodeContent::Branch(mut left, mut right) = parent_raw.content {
                        let left_raw = left.as_mut();
                        let right_raw = right.as_mut();

                        if depth_changed {
                            // rebalance tree if needed
                            let balance = (left_raw.depth as i32) - (right_raw.depth as i32);
                            if balance > 1 {
                                parent_raw.rebalance(false);
                            } else if balance < -1 {
                                parent_raw.rebalance(true);
                            }

                            // check if depth changed
                            let new_depth = left_raw.depth.max(right_raw.depth) + 1;
                            depth_changed = parent_raw.depth != new_depth;
                            parent_raw.depth = new_depth;
                        }

                        // rebalancing shouldnt change bounds
                        // if bounds_changed {}
                    } else {
                        panic!("Parent should always be branch")
                    }

                    if !depth_changed {
                        break;
                    }
                    last_node = parent_raw;
                }
            }
        } else {
            self.root = Some(leaf_ref.leaf);
        }

        self.size += 1;
        Ok(leaf_ref.convert())
    }

    pub fn remove(
        &mut self,
        mut leaf_ref: LeafInHierachy,
    ) -> Result<LeafOutsideHierachy, LeafInHierachy> {
        if leaf_ref.hierachy != self {
            // Leaf does not belong to this hierachy
            return Err(leaf_ref);
        }

        unsafe {
            let leaf_node = leaf_ref.leaf.as_mut();

            // convert parent branch to leaf
            if let Some(parent) = leaf_node.parent {
                let raw_parent = parent.as_ref();
                let mut sibling_leaf = if let NodeContent::Branch(left, right) = raw_parent.content
                {
                    if leaf_node.right_child {
                        left
                    } else {
                        right
                    }
                } else {
                    panic!("Parent should always be branch")
                };

                let raw_sibling = sibling_leaf.as_mut();

                // replace parent with sibling leaf in grandparent
                if let Some(mut grandparent) = raw_parent.parent {
                    // replace grandparent content
                    let raw_grandparent = grandparent.as_mut();
                    if let NodeContent::Branch(left, right) = raw_grandparent.content {
                        if raw_parent.right_child {
                            raw_grandparent.content = NodeContent::Branch(left, sibling_leaf);
                            raw_sibling.right_child = true;
                        } else {
                            raw_grandparent.content = NodeContent::Branch(sibling_leaf, right);
                            raw_sibling.right_child = false;
                        }
                    } else {
                        panic!("Parent should always be branch")
                    };
                } else {
                    // parent is root
                    self.root = Some(sibling_leaf);
                }
                raw_sibling.parent = raw_parent.parent;

                // drop parent
                let _ = Box::from_raw(parent.as_ptr());

                // update parent depth and bounds
                let mut last_node = raw_sibling;
                let mut depth_changed = true;
                let mut bounds_changed = true;
                while let Some(mut parent) = last_node.parent {
                    let parent_raw = parent.as_mut();

                    if let NodeContent::Branch(mut left, mut right) = parent_raw.content {
                        let left_raw = left.as_mut();
                        let right_raw = right.as_mut();

                        if depth_changed {
                            // rebalance tree if needed
                            let balance = (left_raw.depth as i32) - (right_raw.depth as i32);
                            if balance > 1 {
                                parent_raw.rebalance(false);
                            } else if balance < -1 {
                                parent_raw.rebalance(true);
                            }

                            // check if depth changed
                            let new_depth = left_raw.depth.max(right_raw.depth) + 1;
                            depth_changed = parent_raw.depth != new_depth;
                            parent_raw.depth = new_depth;
                        }

                        if bounds_changed {
                            let new_bounds = left_raw.bounds.join(right_raw.bounds);

                            bounds_changed = new_bounds != parent_raw.bounds;
                            parent_raw.bounds = new_bounds;
                        }
                    } else {
                        panic!("Parent should always be branch")
                    }

                    if !depth_changed && !bounds_changed {
                        break;
                    }

                    last_node = parent_raw;
                }
            } else {
                // leaf on root
                self.root = None;
            }
            // Orphan leaf
            leaf_node.parent = None;
        }

        self.size -= 1;
        Ok(leaf_ref.convert())
    }

    pub fn get_overlaps(&self) {}

    pub unsafe fn get_root(&self) -> Option<NonNull<Node>> {
        self.root
    }
}

impl Drop for BVH {
    fn drop(&mut self) {
        // manually drop each node
        if let Some(mut root) = self.root {
            unsafe {
                let mut current = root.as_mut();
                let mut branch_stack: Vec<&mut Node> = Vec::with_capacity(current.depth);

                while self.size > 0 {
                    // check node contents before dropping
                    if let NodeContent::Branch(mut left, mut right) = current.content {
                        // drop node
                        let _ = Box::from_raw(current);

                        // set right child as next
                        branch_stack.push(left.as_mut());
                        current = right.as_mut();
                    } else {
                        // drop node
                        let _ = Box::from_raw(current);

                        // get next node from stack
                        self.size -= 1;
                        if let Some(node) = branch_stack.pop() {
                            current = node;
                        } else {
                            // check if any leaves left that we somehow missed
                            if self.size > 0 {
                                println!(
                                    "Dropping BVH concluded when there are still leaves left to drop"
                                );
                            }
                            break;
                        }
                    };
                }
            }
        }
    }
}

impl Node {
    fn new(collider: Arc<CuboidCollider>) -> NonNull<Self> {
        unsafe {
            NonNull::new_unchecked(Box::into_raw(Box::new(Node {
                parent: None,
                right_child: false,
                bounds: collider.bounding_box,
                depth: 0,
                content: NodeContent::Leaf(collider),
            })))
        }
    }

    fn rebalance(&mut self, right_bigger: bool) {
        if right_bigger {
            // lol
        }
    }
}

impl Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.content {
            NodeContent::Branch(left, right) => unsafe {
                f.debug_struct("Branch")
                    .field("parent", &self.parent)
                    .field("right_child", &self.right_child)
                    .field("depth", &self.depth)
                    .field("left", left.as_ref())
                    .field("right", right.as_ref())
                    .finish()
            },
            NodeContent::Leaf(collider) => f
                .debug_struct("Leaf")
                .field("parent", &self.parent)
                .field("right_child", &self.right_child)
                .field("depth", &self.depth)
                .field("collider", &collider)
                .finish(),
        }
    }
}

// impl<T> LeafReference<T> {
//     fn get_leaf(&self, hierachy: &BVH) -> Option<&Node> {
//         if self.hierachy == hierachy {
//             unsafe { Some(self.leaf.as_ref()) }
//         } else {
//             None
//         }
//     }
//     fn get_leaf_mut(&mut self, hierachy: &mut BVH) -> Option<&mut Node> {
//         if self.hierachy == hierachy {
//             unsafe { Some(self.leaf.as_mut()) }
//         } else {
//             None
//         }
//     }
// }
impl LeafInHierachy {
    fn convert(self) -> LeafOutsideHierachy {
        LeafOutsideHierachy {
            leaf: self.leaf,
            hierachy: self.hierachy,
        }
    }
}
impl LeafOutsideHierachy {
    fn convert(self) -> LeafInHierachy {
        let x = ManuallyDrop::new(self);
        LeafInHierachy {
            leaf: x.leaf,
            hierachy: x.hierachy,
        }
    }
}

impl Debug for LeafInHierachy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Leaf does not belong to this hierachy")
    }
}
impl Debug for LeafOutsideHierachy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Leaf does not belong to this hierachy")
    }
}
// if a ref to a leaf outside the hierachy is dropped we drop the inner leaf node as well
impl Drop for LeafOutsideHierachy {
    fn drop(&mut self) {
        unsafe {
            let _ = Box::from_raw(self.leaf.as_ptr());
        }
    }
}

#[cfg(test)]
mod tree_tests {
    use std::{ptr, sync::Arc};

    use crate::{game_objects::transform::TransformSystem, physics::collider::CuboidCollider};

    use super::{Node, NodeContent, BVH};

    fn validate_tree(child: &Node, parent: Option<&Node>, right_child: bool) -> Result<(), String> {
        // error format is {expected} vs {actual}

        // parent check
        if let Some(actual_parent) = child.parent {
            if let Some(expected_parent) = parent {
                if !ptr::eq(expected_parent, actual_parent.as_ptr()) {
                    return Err(format!("Mismatching parent"));
                }
            } else {
                return Err(format!("Parent present when None was expected"));
            }
        } else if parent.is_some() {
            return Err(format!("Child parent incorrectly set to None"));
        }

        // side check
        if child.right_child != right_child {
            return Err(format!(
                "Incorrect child side: {} vs {}",
                right_child, child.right_child
            ));
        }

        match &child.content {
            NodeContent::Branch(left, right) => unsafe {
                let left_raw = left.as_ref();
                let right_raw = right.as_ref();

                // bounds check
                if child.bounds != left_raw.bounds.join(right_raw.bounds) {
                    return Err(format!(
                        "Incorrect Bounds: {:?} vs {:?}",
                        left_raw.bounds.join(right_raw.bounds),
                        child.bounds
                    ));
                }

                // depth check
                if child.depth != left_raw.depth.max(right_raw.depth) + 1 {
                    return Err(format!(
                        "Incorrect depth: {:?} vs {:?}",
                        left_raw.depth.max(right_raw.depth) + 1,
                        child.depth
                    )
                    .to_string());
                }

                // // balance check
                // if left_raw.depth.abs_diff(right_raw.depth) > 1 {
                //     return Err(format!("Unbalanced tree"));
                // }

                validate_tree(left_raw, Some(child), false)?;
                validate_tree(right_raw, Some(child), true)?;
            },
            NodeContent::Leaf(collider) => {
                // bounds check
                if child.bounds != collider.bounding_box {
                    return Err(format!(
                        "Incorrect Bounds: {:?} vs {:?}",
                        collider.bounding_box, child.bounds
                    ));
                }

                // depth check
                if child.depth != 0 {
                    return Err(format!("Incorrect depth: 0 vs {:?}", child.depth).to_string());
                }
            }
        }

        Ok(())
    }

    fn assert_valid_tree(root: &Node) {
        let validation = validate_tree(&root, None, false);

        assert!(
            validation.is_ok(),
            "Err: {:?}, \nTree: {:#?}",
            validation,
            root
        );
        // assert_eq!("ALL", "GOOD", "Tree: \n{:#?}", root);
    }

    #[test]
    fn insert_test() {
        let mut trans = TransformSystem::new();
        let mut tree = BVH::new();

        let crap_box = super::BoundingBox {
            max: (1.0, 1.0, 1.0).into(),
            min: (0.0, 0.0, 0.0).into(),
        };
        let box_2 = super::BoundingBox {
            max: (2.0, 2.0, 2.0).into(),
            min: (1.0, 1.0, 1.0).into(),
        };

        let a = tree.register_collider(Arc::new(super::CuboidCollider {
            transform: trans.next().unwrap(),
            bounding_box: crap_box,
        }));
        let b = tree.register_collider(Arc::new(super::CuboidCollider {
            transform: trans.next().unwrap(),
            bounding_box: box_2,
        }));
        let c = tree.register_collider(Arc::new(super::CuboidCollider {
            transform: trans.next().unwrap(),
            bounding_box: box_2,
        }));

        let _a = tree.insert(a).unwrap();
        let _b = tree.insert(b).unwrap();
        let _c = tree.insert(c).unwrap();

        unsafe {
            assert_valid_tree(&tree.root.unwrap().as_ref());
        }
    }
    #[test]
    fn remove_test() {
        let mut trans = TransformSystem::new();
        let mut tree = BVH::new();

        let crap_box = super::BoundingBox {
            max: (1.0, 1.0, 1.0).into(),
            min: (0.0, 0.0, 0.0).into(),
        };
        let box_2 = super::BoundingBox {
            max: (2.0, 2.0, 2.0).into(),
            min: (1.0, 1.0, 1.0).into(),
        };

        let a = tree.register_collider(Arc::new(super::CuboidCollider {
            transform: trans.next().unwrap(),
            bounding_box: crap_box,
        }));
        let b = tree.register_collider(Arc::new(super::CuboidCollider {
            transform: trans.next().unwrap(),
            bounding_box: box_2,
        }));
        let c = tree.register_collider(Arc::new(super::CuboidCollider {
            transform: trans.next().unwrap(),
            bounding_box: box_2,
        }));

        let _a = tree.insert(a).unwrap();
        let b = tree.insert(b).unwrap();
        let _c = tree.insert(c).unwrap();

        tree.remove(b).expect("Incorrect hierachy for removal");

        unsafe {
            assert_valid_tree(&tree.root.unwrap().as_ref());
        }
    }
    #[test]
    fn big_tree() {
        let mut trans = TransformSystem::new();
        let mut tree = BVH::new();

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
            let leaf = tree.register_collider(Arc::new(CuboidCollider {
                transform: trans.next().unwrap(),
                bounding_box,
            }));
            tree.insert(leaf).unwrap();
        }

        println!("Yes?");

        unsafe {
            assert_valid_tree(&tree.root.unwrap().as_ref());
        }
    }
    #[test]
    fn big_remove() {
        let mut trans = TransformSystem::new();
        let mut tree = BVH::new();

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

        let leaf = tree.register_collider(Arc::new(CuboidCollider {
            transform: trans.next().unwrap(),
            bounding_box: box_6,
        }));
        let a = tree.insert(leaf).unwrap();

        for bounding_box in [
            crap_box, box_2, box_3, box_4, crap_box, box_5, box_6, box_2, box_4, box_6,
        ] {
            let leaf = tree.register_collider(Arc::new(CuboidCollider {
                transform: trans.next().unwrap(),
                bounding_box,
            }));
            tree.insert(leaf).unwrap();
        }

        let leaf = tree.register_collider(Arc::new(CuboidCollider {
            transform: trans.next().unwrap(),
            bounding_box: box_2,
        }));
        let b = tree.insert(leaf).unwrap();

        tree.remove(a).unwrap();
        tree.remove(b).unwrap();

        unsafe {
            assert_valid_tree(&tree.root.unwrap().as_ref());
        }
    }
    #[test]
    fn remove_branch_root() {
        let mut trans = TransformSystem::new();
        let mut tree = BVH::new();

        let crap_box = super::BoundingBox {
            max: (1.0, 1.0, 1.0).into(),
            min: (0.0, 0.0, 0.0).into(),
        };
        let box_2 = super::BoundingBox {
            max: (2.0, 2.0, 2.0).into(),
            min: (1.0, 1.0, 1.0).into(),
        };

        let a = tree.register_collider(Arc::new(super::CuboidCollider {
            transform: trans.next().unwrap(),
            bounding_box: crap_box,
        }));
        tree.insert(a).unwrap();
        let b = tree.register_collider(Arc::new(super::CuboidCollider {
            transform: trans.next().unwrap(),
            bounding_box: box_2,
        }));
        let b = tree.insert(b).unwrap();

        tree.remove(b).unwrap();
        unsafe {
            assert_valid_tree(&tree.root.unwrap().as_ref());
        }
    }
    #[test]
    fn remove_leaf_root() {
        let mut trans = TransformSystem::new();
        let mut tree = BVH::new();

        let crap_box = super::BoundingBox {
            max: (1.0, 1.0, 1.0).into(),
            min: (0.0, 0.0, 0.0).into(),
        };
        let remove = tree.register_collider(Arc::new(super::CuboidCollider {
            transform: trans.next().unwrap(),
            bounding_box: crap_box,
        }));
        let remove = tree.insert(remove).unwrap();

        tree.remove(remove).unwrap();
        assert!(tree.root.is_none());
        // assert_depth(0, tree.root.unwrap());
    }
    // #[test]
    // fn merge_test() {
    //     let mut trans = TransformSystem::new();
    //     let mut tree1 = BoundsTree::new();
    //     let mut tree2 = BoundsTree::new();

    //     let crap_box = super::BoundingBox {
    //         max: (1.0, 1.0, 1.0).into(),
    //         min: (0.0, 0.0, 0.0).into(),
    //     };
    //     let box_2 = super::BoundingBox {
    //         max: (2.0, 2.0, 2.0).into(),
    //         min: (1.0, 1.0, 1.0).into(),
    //     };
    //     let box_3 = super::BoundingBox {
    //         max: (5.0, 5.0, 6.0).into(),
    //         min: (4.0, 2.0, 4.0).into(),
    //     };
    //     let box_4 = super::BoundingBox {
    //         max: (1.0, 1.0, 1.0).into(),
    //         min: (-1.0, -1.0, -1.0).into(),
    //     };
    //     let box_5 = super::BoundingBox {
    //         max: (0.0, -1.0, 0.0).into(),
    //         min: (-5.0, -2.0, -5.0).into(),
    //     };
    //     let box_6 = super::BoundingBox {
    //         max: (5.0, -1.0, 20.0).into(),
    //         min: (2.0, -5.0, 5.0).into(),
    //     };

    //     for bounding_box in [crap_box, box_2, box_5, box_6, box_2, box_4, box_6] {
    //         tree1.insert_new(CuboidCollider {
    //             transform: trans.next().unwrap(),
    //             bounding_box,
    //         });
    //     }
    //     for bounding_box in [box_5, box_2, box_3] {
    //         tree2.insert_new(CuboidCollider {
    //             transform: trans.next().unwrap(),
    //             bounding_box,
    //         });
    //     }
    //     let uwu = tree2.insert_new(CuboidCollider {
    //         transform: trans.next().unwrap(),
    //         bounding_box: crap_box,
    //     });

    //     tree1.merge(tree2);

    //     tree1.remove(&uwu);

    //     assert_valid_tree(&tree1.root.unwrap());
    // }
    #[test]
    fn removal_balance() {}
}
