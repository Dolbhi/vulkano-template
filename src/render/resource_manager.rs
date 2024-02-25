use std::{collections::HashMap, iter::zip, path::Path, sync::Arc};

use vulkano::image::view::ImageView;

use crate::{vulkano_objects::buffers::Buffers, VertexFull};

use super::{mesh::from_obj, render_data::texture::load_texture, Context};

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub enum MeshID {
    Square,
    Cube,
    Suzanne,
    InaBody,
    InaCloth,
    InaHair,
    InaHead,
    LostEmpire(u8),
    Engine,
    Gun,
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub enum MaterialID {
    LitTexture(TextureID),
    UnlitColor([u8; 4]),
    UV,
    Gradient,
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub enum TextureID {
    InaBody,
    InaCloth,
    InaHair,
    InaHead,
    LostEmpire,
}

pub struct ResourceManager {
    loaded_meshes: HashMap<MeshID, Arc<Buffers<VertexFull>>>,
    loaded_materials: HashMap<MaterialID, Arc<Buffers<VertexFull>>>,
    loaded_textures: HashMap<TextureID, Arc<ImageView>>,
}

impl ResourceManager {
    pub fn begin_loading<'a>(&'a mut self, context: &'a Context) -> ResourceLoader {
        ResourceLoader {
            loaded_resources: self,
            context,
        }
    }
}

pub struct ResourceLoader<'a> {
    loaded_resources: &'a mut ResourceManager,
    context: &'a Context,
}

impl<'a> ResourceLoader<'a> {
    pub fn get_mesh(&mut self, id: MeshID) -> Arc<Buffers<VertexFull>> {
        let loaded_meshes = &mut self.loaded_resources.loaded_meshes;
        match loaded_meshes.get(&id) {
            Some(mesh) => mesh.clone(),
            None => {
                // load mesh
                match id {
                    MeshID::Square => {
                        let vertices = vec![
                            VertexFull {
                                position: [-0.25, -0.25, 0.0],
                                normal: [0.0, 0.0, 1.0],
                                colour: [0.0, 1.0, 0.0],
                                uv: [0.0, 0.0],
                            },
                            VertexFull {
                                position: [0.25, -0.25, 0.0],
                                normal: [0.0, 0.0, 1.0],
                                colour: [0.0, 1.0, 0.0],
                                uv: [1.0, 0.0],
                            },
                            VertexFull {
                                position: [-0.25, 0.25, 0.0],
                                normal: [0.0, 0.0, 1.0],
                                colour: [0.0, 1.0, 0.0],
                                uv: [0.0, 1.0],
                            },
                            VertexFull {
                                position: [0.25, 0.25, 0.0],
                                normal: [0.0, 0.0, 1.0],
                                colour: [0.0, 1.0, 0.0],
                                uv: [1.0, 1.0],
                            },
                        ];
                        let indices = vec![0, 1, 2, 2, 1, 3];
                        let mesh = Arc::new(Buffers::initialize_device_local(
                            &self.context.allocators,
                            self.context.queue.clone(),
                            vertices,
                            indices,
                        ));
                        loaded_meshes.insert(id, mesh);
                    }
                    MeshID::InaBody | MeshID::InaCloth | MeshID::InaHair | MeshID::InaHead => {
                        for (i, mesh) in zip(
                            [
                                MeshID::InaHair,
                                MeshID::InaCloth,
                                MeshID::InaBody,
                                MeshID::InaHead,
                            ],
                            Self::mesh_from_file(&self.context, "models/ina/ReadyToRigINA.obj")
                                .skip(2),
                        ) {
                            loaded_meshes.insert(i, mesh);
                        }
                    }
                    MeshID::LostEmpire(_) => {
                        for (i, mesh) in
                            Self::mesh_from_file(&self.context, "models/lost_empire.obj")
                                .enumerate()
                        {
                            loaded_meshes.insert(MeshID::LostEmpire(i as u8), mesh);
                        }
                    }
                    _ => {
                        let path = match id {
                            MeshID::Cube => "models/default_cube.obj",
                            MeshID::Suzanne => "models/suzanne.obj",
                            MeshID::Engine => "models/engine.obj",
                            MeshID::Gun => "models/gun.obj",
                            _ => panic!("Unmatched mesh id"),
                        };
                        loaded_meshes.insert(
                            id,
                            Self::mesh_from_file(&self.context, path).next().unwrap(),
                        );
                    }
                };
                // try fetch again
                loaded_meshes[&id].clone()
            }
        }
    }

    pub fn get_texture(&mut self, id: TextureID) -> Arc<ImageView> {
        match self.loaded_resources.loaded_textures.get(&id) {
            Some(tex) => tex.clone(),
            None => {
                let path = match id {
                    TextureID::InaBody => "models/ina/Body_Base_Color.png",
                    TextureID::InaCloth => "models/ina/Cloth_Base_Color.png",
                    TextureID::InaHair => "models/ina/Hair_Base_Color.png",
                    TextureID::InaHead => "models/ina/Head_Base_Color.png",
                    TextureID::LostEmpire => "models/lost_empire-RGBA.png",
                };
                let tex = load_texture(
                    &self.context.allocators,
                    &self.context.queue,
                    Path::new(path),
                );
                self.loaded_resources
                    .loaded_textures
                    .insert(id, tex.clone());
                tex
            }
        }
    }

    fn mesh_from_file(
        context: &'a Context,
        path: &str,
    ) -> impl Iterator<Item = Arc<Buffers<VertexFull>>> + 'a {
        from_obj(Path::new(path))
            .into_iter()
            .map(|(vertices, indices)| {
                Arc::new(Buffers::initialize_device_local(
                    &context.allocators,
                    context.queue.clone(),
                    vertices,
                    indices,
                ))
            })
    }
}
