use std::collections::{HashMap, HashSet};

use cgmath::{Matrix4, One, Quaternion, SquareMatrix, Vector3, Zero};

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct TransformID(u32);

#[derive(Clone)]
pub struct Transform {
    parent: Option<TransformID>,
    children: HashSet<TransformID>,
    local_model: Option<Matrix4<f32>>,
    global_model: Option<Matrix4<f32>>,
    translation: Vector3<f32>,
    rotation: Quaternion<f32>,
    scale: Vector3<f32>,
}
impl Transform {
    fn get_local_model(&mut self) -> Matrix4<f32> {
        match self.local_model {
            Some(matrix) => matrix,
            None => {
                // calc model and update
                let model = Matrix4::from_translation(self.translation)
                    * Matrix4::from(self.rotation)
                    * Matrix4::from_nonuniform_scale(self.scale.x, self.scale.y, self.scale.z);

                self.local_model = Some(model);
                model
            }
        }
    }

    /// update global model of self if needed and return global model
    fn clean(&mut self, parent_global: &Matrix4<f32>) -> Matrix4<f32> {
        match self.global_model {
            Some(model) => model,
            None => {
                let new_global = *parent_global * self.get_local_model();
                self.global_model = Some(new_global);
                new_global
            }
        }
    }

    pub fn get_local_transform(&self) -> TransformView {
        TransformView {
            translation: &self.translation,
            rotation: &self.rotation,
            scale: &self.scale,
        }
    }

    fn get_global_translation(&self, parent_matrix: &Matrix4<f32>) -> Vector3<f32> {
        let pos = parent_matrix * self.translation.extend(1.0);
        [pos.x, pos.y, pos.z].map(|v| v / pos.w).into()
    }

    pub fn set_translation(&mut self, translation: impl Into<Vector3<f32>>) -> &mut Self {
        self.translation = translation.into();
        self.local_model = None;
        self.global_model = None;
        self
    }
    pub fn set_rotation(&mut self, rotation: impl Into<Quaternion<f32>>) -> &mut Self {
        self.rotation = rotation.into();
        self.local_model = None;
        self.global_model = None;
        self
    }
    pub fn set_scale(&mut self, scale: impl Into<Vector3<f32>>) -> &mut Self {
        self.scale = scale.into();
        self.local_model = None;
        self.global_model = None;
        self
    }
}

pub struct TransformCreateInfo {
    pub parent: Option<TransformID>,
    pub translation: Vector3<f32>,
    pub rotation: Quaternion<f32>,
    pub scale: Vector3<f32>,
}
impl Into<Transform> for TransformCreateInfo {
    fn into(self) -> Transform {
        Transform {
            parent: self.parent,
            children: HashSet::new(),
            local_model: None,
            global_model: None,
            translation: self.translation,
            rotation: self.rotation,
            scale: self.scale,
        }
    }
}
impl Default for TransformCreateInfo {
    fn default() -> Self {
        Self {
            parent: None,
            translation: Zero::zero(),
            rotation: One::one(),
            scale: Vector3::new(1., 1., 1.),
        }
    }
}

#[derive(Debug)]
pub struct TransformView<'a> {
    pub translation: &'a Vector3<f32>,
    pub rotation: &'a Quaternion<f32>,
    pub scale: &'a Vector3<f32>,
}

#[derive(Debug)]
pub enum TransformError {
    IDNotFound,
}
pub struct TransformSystem {
    root: HashSet<TransformID>,
    transforms: HashMap<TransformID, Transform>,
    next_id: u32,
}
impl TransformSystem {
    pub fn new() -> Self {
        Self {
            root: HashSet::new(),
            transforms: HashMap::new(),
            next_id: 0,
        }
    }

    pub fn get_global_model(&mut self, id: &TransformID) -> Result<Matrix4<f32>, TransformError> {
        let transform = self.transforms.get(id).ok_or(TransformError::IDNotFound)?;

        transform
            .global_model
            .ok_or(TransformError::IDNotFound)
            .or({
                let parent_model = match transform.parent {
                    Some(parent_id) => self.get_global_model(&parent_id)?,
                    None => Matrix4::identity(),
                };

                Ok(self.transforms.get_mut(id).unwrap().clean(&parent_model))
            })
    }
    pub fn get_parent_model(&mut self, id: &TransformID) -> Result<Matrix4<f32>, TransformError> {
        let transform = self.transforms.get(id).ok_or(TransformError::IDNotFound)?;
        if let Some(id) = transform.parent {
            self.get_global_model(&id)
        } else {
            Ok(Matrix4::identity())
        }
    }

    /// Get corresponding transform's position in global space
    pub fn get_global_position(
        &mut self,
        id: &TransformID,
    ) -> Result<Vector3<f32>, TransformError> {
        let parent_model = self.get_parent_model(id)?;
        Ok(self
            .get_transform(id)
            .ok_or(TransformError::IDNotFound)?
            .get_global_translation(&parent_model))
    }

    /// Flag the global model of the corresponding transform and all its children as dirty
    ///
    /// Dirty models are recalculated when they are next retrived
    fn dirty(&mut self, id: &TransformID) -> Result<(), TransformError> {
        let transform = self
            .transforms
            .get_mut(id)
            .ok_or(TransformError::IDNotFound)?;
        transform.global_model = None;

        for child in transform.children.clone() {
            self.dirty(&child)?;
        }

        Ok(())
    }

    /// adds given transform to the system and returns its unique ID
    pub fn add_transform(&mut self, info: TransformCreateInfo) -> TransformID {
        // create id
        let id = TransformID(self.next_id);

        // add to parent
        match info.parent {
            Some(parent_id) => self
                .transforms
                .get_mut(&parent_id)
                .unwrap()
                .children
                .insert(id),
            None => self.root.insert(id),
        };

        // create transform
        self.transforms.insert(id, info.into());
        self.next_id += 1;

        id
    }
    /// swaps parent of child with given parent, dirtying the child accordingly
    pub fn set_parent(
        &mut self,
        child: &TransformID,
        parent: Option<TransformID>,
    ) -> Result<(), TransformError> {
        // update child
        let child_trans = self
            .transforms
            .get_mut(child)
            .ok_or(TransformError::IDNotFound)?;
        let old_parent = child_trans.parent;
        child_trans.parent = parent;
        let _ = self.dirty(child);

        // update old parent
        match old_parent {
            Some(parent_id) => self
                .transforms
                .get_mut(&parent_id)
                .ok_or(TransformError::IDNotFound)?
                .children
                .remove(child),
            None => self.root.remove(child),
        };

        // update new parent
        match parent {
            Some(parent_id) => self
                .transforms
                .get_mut(&parent_id)
                .ok_or(TransformError::IDNotFound)?
                .children
                .insert(*child),
            None => self.root.insert(*child),
        };

        Ok(())
    }

    /// get an immutable view of local transform values
    pub fn get_transform(&self, id: &TransformID) -> Option<&Transform> {
        self.transforms.get(id)
    }
    /// Get mutable reference to the corresponding transform, automatically sets transform and its children to dirty
    pub fn get_transform_mut(&mut self, id: &TransformID) -> Option<&mut Transform> {
        self.dirty(id).ok()?;
        self.transforms.get_mut(id)
    }
}
impl Iterator for TransformSystem {
    type Item = TransformID;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.add_transform(Default::default()))
    }
}
