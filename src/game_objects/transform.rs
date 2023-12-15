use std::collections::HashMap;

use cgmath::{Matrix4, Quaternion, SquareMatrix, Vector3, Zero};

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct TransformID(u32);

#[derive(Clone)]
pub struct Transform {
    parent: Option<TransformID>,
    local_model: Option<Matrix4<f32>>,
    translation: Vector3<f32>,
    rotation: Quaternion<f32>,
    scale: Vector3<f32>,
}
impl Transform {
    pub fn new(create_info: TransformCreateInfo) -> Self {
        let TransformCreateInfo {
            parent,
            translation,
            rotation,
            scale,
        } = create_info;

        Transform {
            local_model: None,
            parent,
            translation,
            rotation,
            scale,
        }
    }

    pub fn get_local_model(&mut self) -> Matrix4<f32> {
        match self.local_model {
            Some(matrix) => matrix,
            None => {
                let model = Matrix4::from_translation(self.translation)
                    * Matrix4::from(self.rotation)
                    * Matrix4::from_nonuniform_scale(self.scale.x, self.scale.y, self.scale.z);
                self.local_model = Some(model);
                model
            }
        }
    }

    pub fn get_transform(&self) -> TransformView {
        TransformView {
            translation: &self.translation,
            rotation: &self.rotation,
            scale: &self.scale,
        }
    }
    pub fn set_parent(&mut self, parent: TransformID) -> Option<TransformID> {
        self.parent.replace(parent)
    }
    pub fn set_translation(&mut self, translation: impl Into<Vector3<f32>>) -> &mut Self {
        self.translation = translation.into();
        self.local_model = None;
        self
    }
    pub fn set_rotation(&mut self, rotation: impl Into<Quaternion<f32>>) -> &mut Self {
        self.rotation = rotation.into();
        self.local_model = None;
        self
    }
    pub fn set_scale(&mut self, scale: impl Into<Vector3<f32>>) -> &mut Self {
        self.scale = scale.into();
        self.local_model = None;
        self
    }

    // pub fn get_parent_mut<'a>(
    //     &self,
    //     transfrom_system: &'a mut TransformSystem,
    // ) -> Option<&'a mut Transform> {
    //     match &self.parent {
    //         Some(id) => transfrom_system.get_transform_mut(id),
    //         None => None,
    //     }
    // }
}

pub struct TransformCreateInfo {
    pub parent: Option<TransformID>,
    pub translation: Vector3<f32>,
    pub rotation: Quaternion<f32>,
    pub scale: Vector3<f32>,
}
impl Into<Transform> for TransformCreateInfo {
    fn into(self) -> Transform {
        Transform::new(self)
    }
}
impl Default for TransformCreateInfo {
    fn default() -> Self {
        Self {
            parent: Default::default(),
            translation: Zero::zero(),
            rotation: Zero::zero(),
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

pub struct TransformSystem {
    transforms: HashMap<TransformID, Transform>,
    next_id: u32,
}
impl TransformSystem {
    pub fn new() -> Self {
        Self {
            transforms: HashMap::new(),
            next_id: 0,
        }
    }

    pub fn get_global_model(&mut self, id: &TransformID) -> Matrix4<f32> {
        let mut current = self
            .get_transform(id)
            .unwrap_or_else(|| panic!("transform system missing given ID"));

        let mut ids = vec![*id];
        while let Some(parent_id) = current.parent {
            ids.push(parent_id);
            current = self
                .get_transform(&parent_id)
                .unwrap_or_else(|| panic!("transform system missing parent ID"));
        }

        let mut model = Matrix4::identity();
        for id in ids {
            model = self.get_transform_mut(&id).unwrap().get_local_model() * model
        }
        model
    }

    pub fn add_transform(&mut self, transform: impl Into<Transform>) -> TransformID {
        let id = TransformID(self.next_id);
        self.transforms.insert(id, transform.into());
        self.next_id += 1;
        id
    }
    pub fn get_transform(&self, id: &TransformID) -> Option<&Transform> {
        self.transforms.get(id)
    }
    pub fn get_transform_mut(&mut self, id: &TransformID) -> Option<&mut Transform> {
        self.transforms.get_mut(id)
    }
}
impl Iterator for TransformSystem {
    type Item = TransformID;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.add_transform(Transform::new(Default::default())))
    }
}
