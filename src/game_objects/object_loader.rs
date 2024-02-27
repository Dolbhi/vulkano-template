use legion::World;

use crate::{
    render::{
        resource_manager::{MaterialID, MeshID, ResourceRetriever},
        RenderObject, RenderSubmit,
    },
    MaterialSwapper,
};

use super::{
    light::{DirectionalLightComponent, PointLightComponent},
    transform::{TransformCreateInfo, TransformID, TransformSystem},
};

#[derive(Clone)]
pub enum ComponentInfo {
    Render(MeshID, MaterialID),
    MaterialSwapper(Vec<MaterialID>),
    PointLight(PointLightComponent),
    DirectionLight(DirectionalLightComponent),
}

impl ComponentInfo {
    fn add_to_entry(&self, entry: &mut legion::world::Entry, resources: &mut ResourceRetriever) {
        match self {
            ComponentInfo::Render(mesh, mat) => {
                let mesh = resources.get_mesh(*mesh);
                let mat = resources.get_material(*mat);
                entry.add_component(RenderObject::new(mesh, mat));
            }
            ComponentInfo::MaterialSwapper(mats) => {
                let mats: Vec<RenderSubmit> = mats
                    .into_iter()
                    .map(|id| resources.get_material(*id))
                    .collect();
                entry.add_component(MaterialSwapper::new(mats));
            }
            ComponentInfo::PointLight(c) => entry.add_component(c.clone()),
            ComponentInfo::DirectionLight(c) => entry.add_component(c.clone()),
        };
    }
}

pub struct ObjectInfo {
    pub transform: TransformCreateInfo,
    pub components: Vec<ComponentInfo>,
    pub children: Vec<ObjectInfo>,
}

impl Default for ObjectInfo {
    fn default() -> Self {
        ObjectInfo {
            transform: Default::default(),
            components: Default::default(),
            children: Default::default(),
        }
    }
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
        // add components
        let transform = self.transforms.add_transform(info.transform);
        let entity = self.world.push((transform.clone(),));
        if let Some(mut entry) = self.world.entry(entity) {
            for component_info in info.components {
                component_info.add_to_entry(&mut entry, &mut self.resource_loader);
            }
        }

        // create children
        for mut child in info.children {
            child.transform.parent = Some(transform.clone());
            let _ = self.create_object(child);
        }

        transform
    }
}
