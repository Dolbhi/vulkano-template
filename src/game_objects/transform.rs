use std::{
    collections::{HashMap, HashSet},
    time::Instant,
};

use cgmath::{Matrix4, One, Quaternion, SquareMatrix, Vector3, VectorSpace, Zero};

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct TransformID(u32);
impl TransformID {
    pub fn id(&self) -> u32 {
        self.0
    }
}

#[derive(Clone)]
pub struct Transform {
    parent: Option<TransformID>,
    children: HashSet<TransformID>,
    local_model: Option<Matrix4<f32>>,
    global_model: Option<Matrix4<f32>>,
    translation: Vector3<f32>,
    rotation: Quaternion<f32>,
    scale: Vector3<f32>,
    last_model: Option<Matrix4<f32>>,
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

    pub fn is_dirty(&self) -> bool {
        self.global_model == None
    }

    pub fn mutate(
        &mut self,
        modification: impl FnOnce(&mut Vector3<f32>, &mut Quaternion<f32>, &mut Vector3<f32>),
    ) {
        modification(&mut self.translation, &mut self.rotation, &mut self.scale);
        self.local_model = None;
        self.global_model = None;
    }

    pub fn get_local_transform(&self) -> TransformView {
        TransformView {
            translation: &self.translation,
            rotation: &self.rotation,
            scale: &self.scale,
        }
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
impl TransformCreateInfo {
    pub fn set_parent(mut self, parent: Option<TransformID>) -> Self {
        self.parent = parent;
        self
    }
    pub fn set_translation(mut self, translation: impl Into<Vector3<f32>>) -> Self {
        self.translation = translation.into();
        self
    }
    pub fn set_rotation(mut self, rotation: impl Into<Quaternion<f32>>) -> Self {
        self.rotation = rotation.into();
        self
    }
    pub fn set_scale(mut self, scale: impl Into<Vector3<f32>>) -> Self {
        self.scale = scale.into();
        self
    }
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
            last_model: None,
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
    last_fixed_time: Instant,
    interpolation: f32,
}
impl TransformSystem {
    pub fn new() -> Self {
        Self {
            root: HashSet::new(),
            transforms: HashMap::new(),
            next_id: 0,
            last_fixed_time: Instant::now(),
            interpolation: 0.0,
        }
    }

    pub fn update_last_fixed(&mut self) {
        self.last_fixed_time = Instant::now();
    }
    pub fn update_interpolation(&mut self, delta_time: f32) -> f32 {
        self.interpolation = (self.last_fixed_time.elapsed().as_secs_f32() / delta_time).min(1.);
        self.interpolation
    }
    pub fn interpolation(&self) -> f32 {
        self.interpolation
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
    // pub fn get_parent_model(&mut self, id: &TransformID) -> Result<Matrix4<f32>, TransformError> {
    //     let transform = self.transforms.get(id).ok_or(TransformError::IDNotFound)?;
    //     if let Some(id) = transform.parent {
    //         self.get_global_model(&id)
    //     } else {
    //         Ok(Matrix4::identity())
    //     }
    // }

    pub fn store_last_model(&mut self, id: &TransformID) -> Result<(), TransformError> {
        let model = self.get_global_model(id)?;
        self.transforms.get_mut(id).unwrap().last_model = Some(model);
        Ok(())
    }
    pub fn clear_last_model(&mut self, id: &TransformID) -> Result<(), TransformError> {
        self.transforms
            .get_mut(id)
            .ok_or(TransformError::IDNotFound)?
            .last_model = None;
        Ok(())
    }
    pub fn get_lerp_model(&mut self, id: &TransformID) -> Result<Matrix4<f32>, TransformError> {
        let now_model = self.get_global_model(id)?;
        let last_model = self.transforms.get_mut(id).unwrap().last_model;
        let model = match last_model {
            None => now_model,
            Some(last_model) => last_model.lerp(now_model, self.interpolation),
        };

        Ok(model)

        //     if let Some(last_model) = last_model {
        //         let old = last_model[3].truncate();
        //         // let new = now_model[3].truncate() / now_model[3][3];
        //         let lerp = model[3].truncate() / model[3][3];
        //         println!(
        //             "[debug] old: {:?}, lerp: {:?}, lerp_v: {:?}",
        //             old.x, lerp.x, self.interpolation
        //         );
        //     }
    }
    // pub fn get_slerp_model(&mut self, id: &TransformID) -> Result<Matrix4<f32>, TransformError> {
    //     let transform = self.transforms.get(id).ok_or(TransformError::IDNotFound)?;
    //     let last_model = transform.last_model;
    //     let parent_model = match transform.parent {
    //         Some(parent_id) => self.get_global_model(&parent_id)?,
    //         None => Matrix4::identity(),
    //     };
    //     let model = match last_model {
    //         None => self
    //             .transforms
    //             .get_mut(id)
    //             .ok_or(TransformError::IDNotFound)?
    //             .clean(&parent_model),
    //         Some(last_model) => {
    //             let view = self.transforms.get(id).unwrap().get_local_transform();

    //             let last_rot: Quaternion<f32> = Matrix3::from_cols(
    //                 last_model[0].truncate(),
    //                 last_model[1].truncate(),
    //                 last_model[2].truncate(),
    //             )
    //             .into();
    //             let last_pos = last_model[3].truncate() / last_model[3][3];

    //             let lerp_rot = last_rot.slerp(*view.rotation, self.interpolation);
    //             let lerp_pos = last_pos.lerp(*view.translation, self.interpolation);

    //             parent_model * Matrix4::from_translation(lerp_pos) * Matrix4::from(lerp_rot)
    //         }
    //     };

    //     Ok(model)
    // }

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
