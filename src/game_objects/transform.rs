use std::collections::HashMap;

use cgmath::{Matrix4, Quaternion, SquareMatrix, Vector3, Zero};

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
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
impl Default for Transform {
    fn default() -> Self {
        Self {
            parent: Default::default(),
            local_model: Default::default(),
            translation: Zero::zero(),
            rotation: Zero::zero(),
            scale: Vector3::new(1., 1., 1.),
        }
    }
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

    pub fn add_transform(&mut self, transform: Transform) -> TransformID {
        let id = TransformID(self.next_id);
        self.transforms.insert(id, transform);
        self.next_id += 1;
        id
    }

    pub fn get_model(&mut self, id: &TransformID) -> Matrix4<f32> {
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
        Some(self.add_transform(Default::default()))
    }
}
