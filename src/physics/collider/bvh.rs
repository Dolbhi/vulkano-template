use super::{ray::Ray, BoundingBox, CuboidCollider};
use crate::{game_objects::transform::TransformSystem, physics::quick_inverse};
use std::{
    fmt::Debug,
    marker::PhantomData,
    mem::{self, ManuallyDrop},
    ptr::NonNull,
};

#[derive(Default)]
pub struct Bvh {
    root: Option<NonNull<Node>>,
    /// number of leafs (excluding those outside hierachy)
    size: usize,
}

pub struct Node {
    parent: Option<NonNull<BranchLinks>>,
    right_child: bool,
    bounds: BoundingBox,
    depth: usize,
    content: NodeContent,
}

enum NodeContent {
    Branch(NonNull<BranchLinks>),
    Leaf(CuboidCollider),
    /// only used for empty root
    None,
}
/// pointers to a set of children and their parent, which in turn have pointers back to the `BranchLink`
struct BranchLinks {
    node: NonNull<Node>,
    left: NonNull<Node>,
    right: NonNull<Node>,
}

/// unique external reference, return to bvh to remove the corresponding leaf node
/// the NonNull node should only be dereferenced when provided with a &mut BVH matching the second element
pub struct LeafInHierachy {
    leaf: NonNull<Node>,
    hierachy: *const Bvh,
}
pub struct LeafOutsideHierachy {
    leaf: NonNull<Node>,
}

/// Depth first iterator for `BoundsTree`, iterates over every node, returning its bounds and depth
pub struct DepthIter<'a> {
    current: Vec<NonNull<Node>>,
    next: Vec<NonNull<Node>>,
    lifetime: PhantomData<&'a Bvh>,
}

impl Bvh {
    pub fn new() -> Self {
        Bvh {
            root: None,
            size: 0,
        }
    }

    pub fn register_collider(collider: CuboidCollider) -> LeafOutsideHierachy {
        LeafOutsideHierachy {
            leaf: Node::new(collider),
        }
    }
    // pub fn deregister_collider(&self, leaf_ref: LeafReference<OutsideHierachy>) {
    //     // properly drop a leaf node
    //     unsafe {
    //         Box::from_raw(leaf_ref.leaf.as_ptr());
    //     }
    // }

    /// traverse down tree to insert leaf before traversing back up tree to update depth and do balancing
    pub fn insert(&mut self, leaf_ref: LeafOutsideHierachy) -> LeafInHierachy {
        if let Some(root) = self.root {
            unsafe {
                let leaf_bounds = leaf_ref.leaf.as_ref().bounds;

                // find best closest leaf
                let mut current = root;
                let mut new_bounds = (*current.as_ptr()).bounds.join(leaf_bounds);

                while let NodeContent::Branch(branch) = (*current.as_ptr()).content {
                    (*current.as_ptr()).bounds = new_bounds;

                    let left_raw = (*branch.as_ptr()).left.as_mut();
                    let right_raw = (*branch.as_ptr()).right.as_mut();

                    let new_left_bounds = left_raw.bounds.join(leaf_bounds);
                    let new_right_bounds = right_raw.bounds.join(leaf_bounds);

                    if new_left_bounds.volume() - left_raw.bounds.volume()
                        <= new_right_bounds.volume() - right_raw.bounds.volume()
                    {
                        current = (*branch.as_ptr()).left;
                        new_bounds = new_left_bounds;
                    } else {
                        current = (*branch.as_ptr()).right;
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
                    content: NodeContent::None,
                })));
                let branch_content = BranchLinks::new(new_branch, current, leaf_ref.leaf);
                (*new_branch.as_ptr()).content = NodeContent::Branch(branch_content);

                (*current.as_ptr()).parent = Some(branch_content);
                (*current.as_ptr()).right_child = false;
                (*leaf_ref.leaf.as_ptr()).parent = Some(branch_content);
                (*leaf_ref.leaf.as_ptr()).right_child = true;

                // update parent's content
                if let Some(mut parent) = parent {
                    if right_child {
                        parent.as_mut().right = new_branch;
                    } else {
                        parent.as_mut().left = new_branch;
                    }
                } else {
                    self.root = Some(new_branch);
                }

                // update parent depths
                let mut last_node = new_branch.as_mut();
                let mut depth_changed = true;
                // let mut bounds_changed = false;
                while let Some(parent) = last_node.parent {
                    // let parent_raw = parent.as_mut();
                    let parent_node = (*parent.as_ptr()).node;

                    if depth_changed {
                        let old_depth = parent_node.as_ref().depth;

                        // rebalance tree if needed
                        (*parent_node.as_ptr()).rebalance();

                        // check if depth changed
                        depth_changed = old_depth != parent_node.as_ref().depth;
                    }

                    if !depth_changed {
                        break;
                    }
                    last_node = &mut *parent_node.as_ptr();
                }
            }
        } else {
            self.root = Some(leaf_ref.leaf);
        }

        self.size += 1;
        leaf_ref.convert(self)
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
            if let Some(parent_content) = leaf_node.parent.take() {
                let raw_parent = parent_content.as_ref();
                let mut sibling_leaf = if leaf_node.right_child {
                    raw_parent.left
                } else {
                    raw_parent.right
                };

                let raw_sibling = sibling_leaf.as_mut();

                // replace parent with sibling leaf in grandparent
                let parent_node = &mut (*raw_parent.node.as_ptr());
                if let Some(mut grandparent) = parent_node.parent {
                    // replace grandparent content
                    let raw_grandparent = grandparent.as_mut();
                    if parent_node.right_child {
                        raw_grandparent.right = sibling_leaf;
                        raw_sibling.right_child = true;
                    } else {
                        raw_grandparent.left = sibling_leaf;
                        raw_sibling.right_child = false;
                    }
                } else {
                    // parent is root
                    self.root = Some(sibling_leaf);
                }
                raw_sibling.parent = parent_node.parent;

                // drop parent
                let _ = Box::from_raw(parent_content.as_ref().node.as_ptr());
                let _ = Box::from_raw(parent_content.as_ptr());

                // update parent depth and bounds
                let mut last_node = sibling_leaf;
                let mut depth_changed = true;
                let mut bounds_changed = true;
                while let Some(parent) = last_node.as_ref().parent {
                    let parent_node = parent.as_ref().node;

                    if depth_changed {
                        let old_depth = parent_node.as_ref().depth;

                        // rebalance tree if needed
                        (*parent_node.as_ptr()).rebalance();

                        // check if depth changed
                        depth_changed = old_depth != parent_node.as_ref().depth;
                    }

                    if bounds_changed {
                        let new_bounds = parent
                            .as_ref()
                            .left
                            .as_ref()
                            .bounds
                            .join(parent.as_ref().right.as_ref().bounds);

                        bounds_changed = new_bounds != parent_node.as_ref().bounds;
                        (*parent_node.as_ptr()).bounds = new_bounds;
                    }

                    if !depth_changed && !bounds_changed {
                        break;
                    }

                    last_node = parent_node;
                }
            } else {
                // leaf on root
                self.root = None;
            }
        }

        self.size -= 1;
        Ok(leaf_ref.convert())
    }

    /// remove and reinsert leaf with modification
    pub fn modify_collider<F>(
        &mut self,
        leaf_ref: &mut LeafInHierachy,
        modification: F,
    ) -> Result<(), ()>
    where
        F: FnOnce(&mut CuboidCollider),
    {
        if leaf_ref.hierachy != self {
            // Leaf does not belong to this hierachy
            Err(())
        } else {
            unsafe {
                // clone leaf (LEAF CLONE IS DANGEROUS DONT LET CLONE AND OG ESCAPE TOGETHER)
                let leaf_clone = leaf_ref.clone();
                // use clone for removal
                let mut res = self.remove(leaf_clone).unwrap();
                // modify collider safely
                modification(res.get_collider_mut());
                // update bounds
                res.leaf.as_mut().bounds = res.get_collider().bounding_box;
                // re-insert leaf and reconcille clone and og (kill og)
                *leaf_ref = self.insert(res);
            }
            Ok(())
        }
    }

    pub fn get_overlaps(&self) -> Vec<(&CuboidCollider, &CuboidCollider)> {
        match self.root {
            Some(root) => unsafe {
                if let NodeContent::Branch(branch) = (*root.as_ptr()).content {
                    let mut results = Vec::with_capacity(self.size);
                    (*branch.as_ptr())
                        .left
                        .as_ref()
                        .check_overlap((*branch.as_ptr()).right.as_ref(), &mut results);
                    results
                } else {
                    vec![]
                }
            },
            None => {
                vec![]
            }
        }
    }

    pub fn raycast(
        &self,
        ray: &Ray,
        transforms: &mut TransformSystem,
    ) -> Option<(f32, &CuboidCollider)> {
        self.root
            .and_then(|root_node| unsafe { root_node.as_ref().raycast(ray, transforms) })
    }

    pub unsafe fn get_root(&self) -> Option<NonNull<Node>> {
        self.root
    }

    pub fn iter(&self) -> DepthIter {
        if let Some(root) = self.root {
            DepthIter {
                current: vec![root],
                next: vec![],
                lifetime: PhantomData,
            }
        } else {
            DepthIter {
                current: vec![],
                next: vec![],
                lifetime: PhantomData,
            }
        }
    }

    pub fn depth(&self) -> usize {
        match self.root {
            Some(root) => unsafe { root.as_ref().depth },
            None => 0,
        }
    }
}
// impl Default for BVH {
//     fn default() -> Self {

//     }
// }
unsafe impl Send for Bvh {}
unsafe impl Sync for Bvh {}

impl Drop for Bvh {
    fn drop(&mut self) {
        // manually drop each node
        if let Some(mut root) = self.root {
            unsafe {
                let mut current = root.as_mut();
                let mut branch_stack: Vec<&mut Node> = Vec::with_capacity(current.depth);

                while self.size > 0 {
                    // check node contents before dropping
                    if let NodeContent::Branch(branch_content) = current.content {
                        // drop node
                        let _ = Box::from_raw(current);

                        // set right child as next
                        branch_stack.push((*branch_content.as_ptr()).left.as_mut());
                        current = (*branch_content.as_ptr()).right.as_mut();

                        // drop branch content
                        let _ = Box::from_raw(branch_content.as_ptr());
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
    fn new(collider: CuboidCollider) -> NonNull<Self> {
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

    /// calculate depths and rebalances tree if needed
    fn rebalance(&mut self) {
        // println!("Right bigger: {}", right_bigger);
        unsafe {
            // determine larger child
            let branch_content = match self.content {
                NodeContent::Branch(branch) => branch,
                NodeContent::Leaf(_) => return,
                NodeContent::None => return,
            };

            let left_depth = branch_content.as_ref().left.as_ref().depth as i32;
            let right_depth = branch_content.as_ref().right.as_ref().depth as i32;

            let balance = left_depth - right_depth;
            let larger_child = if balance > 1 {
                branch_content.as_ref().left
            } else if balance < -1 {
                branch_content.as_ref().right
            } else {
                self.depth = (left_depth.max(right_depth) + 1) as usize;
                return;
            };

            // determine larger grandchild in larger child
            let child_content = if let NodeContent::Branch(branch) = larger_child.as_ref().content {
                branch
            } else {
                panic!("During rebalancing, found larger node to have actual depth < 2");
            };

            let branch_raw = &mut *branch_content.as_ptr();
            let child_raw = &mut *child_content.as_ptr();

            let small_child = if larger_child.as_ref().right_child {
                &mut branch_raw.left
            } else {
                &mut branch_raw.right
            };
            let large_grand = if child_raw.left.as_ref().depth < child_raw.right.as_ref().depth {
                &mut child_raw.right
            } else {
                &mut child_raw.left
            };

            // swap smaller child and larger grandchild's positions
            mem::swap(small_child, large_grand);
            mem::swap(
                &mut small_child.as_mut().right_child,
                &mut large_grand.as_mut().right_child,
            );
            mem::swap(
                &mut small_child.as_mut().parent,
                &mut large_grand.as_mut().parent,
            );

            // update larger child's depth and bounds
            child_raw.recalculate_depth();
            child_raw.recalculate_bounds();

            // update self depth (asumming an initial imbalance of 2)
            self.depth = (left_depth as usize + right_depth as usize) / 2 + 1;
        }
    }

    fn check_overlap<'a>(
        &'a self,
        other: &'a Node,
        results: &mut Vec<(&'a CuboidCollider, &'a CuboidCollider)>,
    ) {
        unsafe {
            // recurse to children if any
            if let NodeContent::Branch(branch) = self.content {
                (*branch.as_ptr())
                    .left
                    .as_ref()
                    .check_overlap((*branch.as_ptr()).right.as_ref(), results);
            }
            if let NodeContent::Branch(other_branch) = other.content {
                (*other_branch.as_ptr())
                    .left
                    .as_ref()
                    .check_overlap((*other_branch.as_ptr()).right.as_ref(), results);
            }

            if self.bounds.check_overlap(other.bounds) {
                match (&self.content, &other.content) {
                    (NodeContent::Branch(branch), NodeContent::Branch(other_branch)) => {
                        // both are branches
                        if self.bounds.volume() > other.bounds.volume() {
                            (*branch.as_ptr())
                                .left
                                .as_ref()
                                .check_overlap(other, results);
                            (*branch.as_ptr())
                                .right
                                .as_ref()
                                .check_overlap(other, results);
                        } else {
                            (*other_branch.as_ptr())
                                .left
                                .as_ref()
                                .check_overlap(self, results);
                            (*other_branch.as_ptr())
                                .right
                                .as_ref()
                                .check_overlap(self, results);
                        }
                    }
                    (NodeContent::Branch(branch), _) => {
                        // self is branch, other is leaf
                        (*branch.as_ptr())
                            .left
                            .as_ref()
                            .check_overlap(other, results);
                        (*branch.as_ptr())
                            .right
                            .as_ref()
                            .check_overlap(other, results);
                    }
                    (_, NodeContent::Branch(other_branch)) => {
                        // self is leaf, other is branch
                        (*other_branch.as_ptr())
                            .left
                            .as_ref()
                            .check_overlap(self, results);
                        (*other_branch.as_ptr())
                            .right
                            .as_ref()
                            .check_overlap(self, results);
                    }
                    (NodeContent::Leaf(coll), NodeContent::Leaf(other_coll)) => {
                        // both are leaves
                        results.push((coll, other_coll))
                    }
                    (_, _) => {
                        println!("NONE NODE IN TREE???");
                    }
                }
            }
        }
    }

    fn raycast(
        &self,
        ray: &Ray,
        transforms: &mut TransformSystem,
    ) -> Option<(f32, &CuboidCollider)> {
        match &self.content {
            NodeContent::Leaf(collider) => {
                let mut model = transforms.get_global_model(&collider.transform).unwrap();
                quick_inverse(&mut model);
                ray.cuboid_intersection(&model)
                    .map(|depth| (depth, collider))
            }
            NodeContent::Branch(branch) => unsafe {
                let left_dist = ray.box_intersection(&branch.as_ref().left.as_ref().bounds);
                let right_dist = ray.box_intersection(&branch.as_ref().right.as_ref().bounds);

                match (left_dist, right_dist) {
                    (Some(left), Some(right)) => {
                        // check first intercepted child first
                        let (close_child, far_child) = if left < right {
                            ((*branch.as_ptr()).left, (*branch.as_ptr()).right)
                        } else {
                            ((*branch.as_ptr()).right, (*branch.as_ptr()).left)
                        };
                        let far_dist = left.max(right);

                        let close_result = close_child.as_ref().raycast(ray, transforms);

                        if let Some((close_dist, _)) = close_result {
                            // check if far bounds is entered before close result
                            if close_dist < far_dist {
                                close_result
                            } else {
                                // try raycast far
                                let far_result = far_child.as_ref().raycast(ray, transforms);
                                far_result.map_or(close_result, |(far_dist, _)| {
                                    if close_dist < far_dist {
                                        close_result
                                    } else {
                                        far_result
                                    }
                                })
                            }
                        } else {
                            far_child.as_ref().raycast(ray, transforms)
                        }
                    }
                    (Some(_), None) => branch.as_ref().left.as_ref().raycast(ray, transforms),
                    (None, Some(_)) => branch.as_ref().right.as_ref().raycast(ray, transforms),
                    _ => None,
                }
            },
            NodeContent::None => {
                println!("NONE NODE IN TREE???");
                None
            }
        }
    }
}

impl Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.content {
            NodeContent::Branch(branch) => unsafe {
                f.debug_struct("Branch")
                    .field("parent", &self.parent)
                    .field("right_child", &self.right_child)
                    .field("depth", &self.depth)
                    .field("left", branch.as_ref().left.as_ref())
                    .field("right", branch.as_ref().right.as_ref())
                    .finish()
            },
            NodeContent::Leaf(collider) => f
                .debug_struct("Leaf")
                .field("parent", &self.parent)
                .field("right_child", &self.right_child)
                .field("depth", &self.depth)
                .field("collider", &collider)
                .finish(),
            NodeContent::None => f.debug_struct("None").finish(),
        }
    }
}

impl BranchLinks {
    fn new(node: NonNull<Node>, left: NonNull<Node>, right: NonNull<Node>) -> NonNull<Self> {
        unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(Self { node, left, right }))) }
    }

    fn recalculate_depth(&mut self) {
        unsafe {
            (*self.node.as_ptr()).depth =
                self.left.as_ref().depth.max(self.right.as_ref().depth) + 1;
        }
    }

    fn recalculate_bounds(&mut self) {
        unsafe {
            (*self.node.as_ptr()).bounds =
                self.left.as_ref().bounds.join(self.right.as_ref().bounds);
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
        LeafOutsideHierachy { leaf: self.leaf }
    }

    // probably nothing wrong with leaving this in
    // pub fn get_collider(&self, hierachy: &BVH) -> Option<&CuboidCollider> {
    //     if ptr::eq(self.hierachy, hierachy) {
    //         unsafe {
    //             if let NodeContent::Leaf(collider) = &self.leaf.as_ref().content {
    //                 Some(collider)
    //             } else {
    //                 None
    //             }
    //         }
    //     } else {
    //         None
    //     }
    // }

    /// Having 2 copies of a leaf reference and using one for removal leaves a LeafInHierachy that references a leaf outside hierachy
    /// Not to mention dropping the newly converted LeafOutsideHierachy leaves dangling pointers
    /// Note: This is used for quick removal and reinsertion without dropping the original LeafInHierachy, if we used a wrapper for the ECS this would not be needed
    unsafe fn clone(&self) -> Self {
        Self {
            leaf: self.leaf,
            hierachy: self.hierachy,
        }
    }
}
impl LeafOutsideHierachy {
    fn convert(self, hierachy: *const Bvh) -> LeafInHierachy {
        let x = ManuallyDrop::new(self);
        LeafInHierachy {
            leaf: x.leaf,
            hierachy,
        }
    }

    pub fn get_collider(&self) -> &CuboidCollider {
        unsafe {
            if let NodeContent::Leaf(collider) = &self.leaf.as_ref().content {
                collider
            } else {
                panic!("Leaf reference does not point to leaf")
            }
        }
    }

    pub fn get_collider_mut(&mut self) -> &mut CuboidCollider {
        unsafe {
            if let NodeContent::Leaf(collider) = &mut self.leaf.as_mut().content {
                collider
            } else {
                panic!("Leaf reference does not point to leaf")
            }
        }
    }
}

impl Debug for LeafInHierachy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Leaf does not belong to this hierachy")
    }
}
unsafe impl Send for LeafInHierachy {}
unsafe impl Sync for LeafInHierachy {}

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

impl<'a> IntoIterator for &'a Bvh {
    type IntoIter = DepthIter<'a>;
    type Item = (BoundingBox, usize);

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a> Iterator for DepthIter<'a> {
    type Item = (BoundingBox, usize);

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            match self.current.pop() {
                Some(node) => {
                    if let NodeContent::Branch(branch) = node.as_ref().content {
                        self.next.push(branch.as_ref().left);
                        self.next.push(branch.as_ref().right);
                    }
                    Some((node.as_ref().bounds, node.as_ref().depth))
                }
                None => {
                    // swap current and next
                    std::mem::swap(&mut self.current, &mut self.next);

                    // try pop current again
                    if let Some(node) = self.current.pop() {
                        if let NodeContent::Branch(branch) = node.as_ref().content {
                            self.next.push(branch.as_ref().left);
                            self.next.push(branch.as_ref().right);
                        }
                        Some((node.as_ref().bounds, node.as_ref().depth))
                    } else {
                        // both vecs empty
                        None
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tree_tests {
    use std::ptr::{self, addr_of, NonNull};

    use crate::{game_objects::transform::TransformSystem, physics::collider::CuboidCollider};

    use super::{BranchLinks, Bvh, Node, NodeContent};

    fn validate_tree(
        child: &Node,
        parent: Option<&NonNull<BranchLinks>>,
        right_child: bool,
    ) -> Result<(), String> {
        // error format is {expected} vs {actual}

        // parent check
        if let Some(actual_parent) = child.parent {
            if let Some(expected_parent) = parent {
                if !ptr::eq(expected_parent.as_ptr(), actual_parent.as_ptr()) {
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
            NodeContent::Branch(branch) => unsafe {
                // check node
                if !ptr::eq(child, branch.as_ref().node.as_ptr()) {
                    return Err(format!(
                        "Incorrect Node in BranchContent: {:?} vs {:?}",
                        addr_of!(child),
                        branch.as_ref().node.as_ptr()
                    ));
                }

                let left_raw = branch.as_ref().left.as_ref();
                let right_raw = branch.as_ref().right.as_ref();

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

                validate_tree(left_raw, Some(branch), false)?;
                validate_tree(right_raw, Some(branch), true)?;
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
            NodeContent::None => {
                return Err(format!("Found unitialised node"));
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
        let mut tree = Bvh::new();

        let crap_box = super::BoundingBox {
            max: (1.0, 1.0, 1.0).into(),
            min: (0.0, 0.0, 0.0).into(),
        };
        let box_2 = super::BoundingBox {
            max: (2.0, 2.0, 2.0).into(),
            min: (1.0, 1.0, 1.0).into(),
        };

        let a = Bvh::register_collider(super::CuboidCollider {
            transform: trans.next().unwrap(),
            bounding_box: crap_box,
            rigidbody: None,
        });
        let b = Bvh::register_collider(super::CuboidCollider {
            transform: trans.next().unwrap(),
            bounding_box: box_2,
            rigidbody: None,
        });
        let c = Bvh::register_collider(super::CuboidCollider {
            transform: trans.next().unwrap(),
            bounding_box: box_2,
            rigidbody: None,
        });

        let _a = tree.insert(a);
        let _b = tree.insert(b);
        let _c = tree.insert(c);

        unsafe {
            assert_valid_tree(&tree.root.unwrap().as_ref());
        }
    }
    #[test]
    fn remove_test() {
        let mut trans = TransformSystem::new();
        let mut tree = Bvh::new();

        let crap_box = super::BoundingBox {
            max: (1.0, 1.0, 1.0).into(),
            min: (0.0, 0.0, 0.0).into(),
        };
        let box_2 = super::BoundingBox {
            max: (2.0, 2.0, 2.0).into(),
            min: (1.0, 1.0, 1.0).into(),
        };

        let a = Bvh::register_collider(super::CuboidCollider {
            transform: trans.next().unwrap(),
            bounding_box: crap_box,
            rigidbody: None,
        });
        let b = Bvh::register_collider(super::CuboidCollider {
            transform: trans.next().unwrap(),
            bounding_box: box_2,
            rigidbody: None,
        });
        let c = Bvh::register_collider(super::CuboidCollider {
            transform: trans.next().unwrap(),
            bounding_box: box_2,
            rigidbody: None,
        });

        let _a = tree.insert(a);
        let b = tree.insert(b);
        let _c = tree.insert(c);

        tree.remove(b).expect("Incorrect hierachy for removal");

        unsafe {
            assert_valid_tree(&tree.root.unwrap().as_ref());
        }
    }
    #[test]
    fn big_tree() {
        let mut trans = TransformSystem::new();
        let mut tree = Bvh::new();

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
            let leaf = Bvh::register_collider(CuboidCollider {
                transform: trans.next().unwrap(),
                bounding_box,
                rigidbody: None,
            });
            tree.insert(leaf);
        }

        println!("Yes?");

        unsafe {
            assert_valid_tree(&tree.root.unwrap().as_ref());
        }
    }
    #[test]
    fn big_remove() {
        let mut trans = TransformSystem::new();
        let mut tree = Bvh::new();

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

        let leaf = Bvh::register_collider(CuboidCollider {
            transform: trans.next().unwrap(),
            bounding_box: box_6,
            rigidbody: None,
        });
        let a = tree.insert(leaf);

        for bounding_box in [
            crap_box, box_2, box_3, box_4, crap_box, box_5, box_6, box_2, box_4, box_6,
        ] {
            let leaf = Bvh::register_collider(CuboidCollider {
                transform: trans.next().unwrap(),
                bounding_box,
                rigidbody: None,
            });
            tree.insert(leaf);
            // unsafe {
            //     println!("{:#?}", tree.root.unwrap().as_ref());
            // }
        }

        let leaf = Bvh::register_collider(CuboidCollider {
            transform: trans.next().unwrap(),
            bounding_box: box_2,
            rigidbody: None,
        });
        let b = tree.insert(leaf);

        tree.remove(a).unwrap();
        tree.remove(b).unwrap();

        unsafe {
            assert_valid_tree(&tree.root.unwrap().as_ref());
        }
    }
    #[test]
    fn remove_branch_root() {
        let mut trans = TransformSystem::new();
        let mut tree = Bvh::new();

        let crap_box = super::BoundingBox {
            max: (1.0, 1.0, 1.0).into(),
            min: (0.0, 0.0, 0.0).into(),
        };
        let box_2 = super::BoundingBox {
            max: (2.0, 2.0, 2.0).into(),
            min: (1.0, 1.0, 1.0).into(),
        };

        let a = Bvh::register_collider(super::CuboidCollider {
            transform: trans.next().unwrap(),
            bounding_box: crap_box,
            rigidbody: None,
        });
        tree.insert(a);
        let b = Bvh::register_collider(super::CuboidCollider {
            transform: trans.next().unwrap(),
            bounding_box: box_2,
            rigidbody: None,
        });
        let b = tree.insert(b);

        tree.remove(b).unwrap();
        unsafe {
            assert_valid_tree(&tree.root.unwrap().as_ref());
        }
    }
    #[test]
    fn remove_leaf_root() {
        let mut trans = TransformSystem::new();
        let mut tree = Bvh::new();

        let crap_box = super::BoundingBox {
            max: (1.0, 1.0, 1.0).into(),
            min: (0.0, 0.0, 0.0).into(),
        };
        let remove = Bvh::register_collider(super::CuboidCollider {
            transform: trans.next().unwrap(),
            bounding_box: crap_box,
            rigidbody: None,
        });
        let remove = tree.insert(remove);

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
