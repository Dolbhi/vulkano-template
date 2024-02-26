use legion::World;

use crate::{
    render::{
        resource_manager::{self, MaterialID, MeshID, ResourceRetriever},
        RenderObject, RenderSubmit,
    },
    MaterialSwapper,
};

use super::{
    light::{DirectionalLightComponent, PointLightComponent},
    transform::{TransformCreateInfo, TransformID, TransformSystem},
};

pub enum ComponentInfo {
    Render(MeshID, MaterialID),
    MaterialSwapper(Vec<MaterialID>),
    PointLight(PointLightComponent),
    DirectionLight(DirectionalLightComponent),
}

pub struct ObjectInfo {
    transform: TransformCreateInfo,
    components: Vec<ComponentInfo>,
}

pub struct ObjectLoader<'a> {
    resource_loader: ResourceRetriever<'a>,
    world: &'a mut World,
    transforms: &'a mut TransformSystem,
}

impl<'a> ObjectLoader<'a> {
    pub fn new(
        resource_loader: ResourceRetriever<'a>,
        world: &'a mut World,
        transforms: &'a mut TransformSystem,
    ) -> Self {
        ObjectLoader {
            resource_loader,
            world,
            transforms,
        }
    }

    pub fn create_object(&mut self, info: ObjectInfo) -> TransformID {
        let transform = self.transforms.add_transform(info.transform);
        let entity = self.world.push((transform.clone(),));
        if let Some(mut entry) = self.world.entry(entity) {
            for component_info in info.components {
                match component_info {
                    ComponentInfo::Render(mesh, mat) => {
                        let mesh = self.resource_loader.get_mesh(mesh);
                        let mat = self.resource_loader.get_material(mat);
                        entry.add_component(RenderObject::new(mesh, mat));
                    }
                    ComponentInfo::MaterialSwapper(mats) => {
                        let mats: Vec<RenderSubmit> = mats
                            .into_iter()
                            .map(|id| self.resource_loader.get_material(id))
                            .collect();
                        entry.add_component(MaterialSwapper::new(mats));
                    }
                    ComponentInfo::PointLight(c) => entry.add_component(c),
                    ComponentInfo::DirectionLight(c) => entry.add_component(c),
                }
            }
        }
        transform
    }
}
