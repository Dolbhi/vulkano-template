use std::{collections::HashMap, iter::zip, path::Path, sync::Arc};

use vulkano::{
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    image::{sampler::Sampler, view::ImageView},
};

use crate::{shaders::draw, vulkano_objects::buffers::Buffers, VertexFull};

use super::{
    mesh::from_obj,
    render_data::{
        material::Shader,
        texture::{create_sampler, load_texture},
    },
    Context, DrawSystem, RenderSubmit,
};

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

const LOST_EMPIRE_MESH_COUNT: u8 = 45;

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum MaterialID {
    LitTexture(TextureID),
    LitColor([u8; 4]),
    UnlitTexture(TextureID),
    UnlitColor([u8; 4]),
    UV,
    Gradient,
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum TextureID {
    InaBody,
    InaCloth,
    InaHair,
    InaHead,
    LostEmpire,
}

pub struct ResourceManager {
    loaded_meshes: HashMap<MeshID, Arc<Buffers<VertexFull>>>,
    loaded_materials: HashMap<MaterialID, RenderSubmit>,
    loaded_textures: HashMap<TextureID, Arc<ImageView>>,
    linear_sampler: Arc<Sampler>,
}

impl ResourceManager {
    pub fn new(context: &Context) -> Self {
        ResourceManager {
            loaded_meshes: HashMap::new(),
            loaded_materials: HashMap::new(),
            loaded_textures: HashMap::new(),
            linear_sampler: create_sampler(
                context.device.clone(),
                vulkano::image::sampler::Filter::Linear,
            ),
        }
    }

    pub fn begin_retrieving<'a>(
        &'a mut self,
        context: &'a Context,
        lit_system: &'a mut DrawSystem,
        unlit_system: &'a mut DrawSystem,
    ) -> ResourceRetriever {
        ResourceRetriever {
            loaded_resources: self,
            context,
            lit_system,
            unlit_system,
        }
    }
}

pub struct ResourceRetriever<'a> {
    loaded_resources: &'a mut ResourceManager,
    context: &'a Context,
    lit_system: &'a mut DrawSystem,
    unlit_system: &'a mut DrawSystem,
}

impl<'a> ResourceRetriever<'a> {
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
                            mesh_from_file(&self.context, "models/ina/ReadyToRigINA.obj").skip(2),
                        ) {
                            loaded_meshes.insert(i, mesh);
                        }
                    }
                    MeshID::LostEmpire(n) => {
                        assert!(
                            n < LOST_EMPIRE_MESH_COUNT,
                            "Lost empire only has 45 sub-meshes"
                        );
                        for (i, mesh) in
                            mesh_from_file(&self.context, "models/lost_empire.obj").enumerate()
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
                        loaded_meshes
                            .insert(id, mesh_from_file(&self.context, path).next().unwrap());
                    }
                };
                // try fetch again
                loaded_meshes[&id].clone()
            }
        }
    }

    pub fn get_material(&mut self, id: MaterialID) -> RenderSubmit {
        match self.loaded_resources.loaded_materials.get(&id) {
            Some(mat) => mat.clone(),
            None => {
                // Narrow down system
                let system = match id {
                    MaterialID::LitTexture(_) | MaterialID::LitColor(_) => &mut self.lit_system,
                    _ => &mut self.unlit_system,
                };
                // Narrow down shader
                let shader = match system.find_shader(id) {
                    Some(s) => s,
                    None => {
                        {
                            // load shader
                            match id {
                                MaterialID::LitTexture(_) => {
                                    panic!("Basic lit shader should be loaded by default")
                                }
                                MaterialID::LitColor(_) => {
                                    system.add_shader(
                                        &self.context,
                                        MaterialID::LitColor([0, 0, 0, 0]),
                                        draw::load_basic_vs(self.context.device.clone())
                                            .expect("failed to create solid shader module"),
                                        draw::load_solid_fs(self.context.device.clone())
                                            .expect("failed to create solid shader module"),
                                    );
                                }
                                MaterialID::UnlitTexture(_) => {
                                    panic!("Lit color shader should be loaded by default")
                                }
                                MaterialID::UnlitColor(_) => {
                                    system.add_shader(
                                        &self.context,
                                        MaterialID::UnlitColor([0, 0, 0, 0]),
                                        draw::load_basic_vs(self.context.device.clone())
                                            .expect("failed to create solid shader module"),
                                        draw::load_solid_fs(self.context.device.clone())
                                            .expect("failed to create solid shader module"),
                                    );
                                }
                                MaterialID::UV => {
                                    system.add_shader(
                                        &self.context,
                                        MaterialID::UV,
                                        draw::load_basic_vs(self.context.device.clone())
                                            .expect("failed to create uv shader module"),
                                        draw::load_uv_fs(self.context.device.clone())
                                            .expect("failed to create uv shader module"),
                                    );
                                }
                                MaterialID::Gradient => {
                                    system.add_shader(
                                        &self.context,
                                        MaterialID::Gradient,
                                        draw::load_basic_vs(self.context.device.clone())
                                            .expect("failed to create grad shader module"),
                                        draw::load_grad_fs(self.context.device.clone())
                                            .expect("failed to create grad shader module"),
                                    );
                                }
                            };
                            system.find_shader(id).unwrap()
                        }
                    }
                };
                // make material
                let material = match id {
                    MaterialID::LitTexture(tex_id) | MaterialID::UnlitTexture(tex_id) => {
                        let tex =
                            Self::get_texture(&mut self.loaded_resources, &self.context, tex_id);
                        init_material(
                            &self.context,
                            shader,
                            [WriteDescriptorSet::image_view_sampler(
                                0,
                                tex,
                                self.loaded_resources.linear_sampler.clone(),
                            )],
                        )
                    }
                    MaterialID::LitColor(color) | MaterialID::UnlitColor(color) => {
                        let color_buffer = create_material_buffer(
                            &self.context,
                            draw::SolidData {
                                color: color.map(|v| (v as f32) / (u8::MAX as f32)),
                            },
                            vulkano::buffer::BufferUsage::empty(),
                        );
                        init_material(
                            &self.context,
                            shader,
                            [WriteDescriptorSet::buffer(0, color_buffer)],
                        )
                    }
                    _ => shader.add_material(None),
                };
                self.loaded_resources
                    .loaded_materials
                    .insert(id, material.clone());
                material
            }
        }
    }

    pub fn get_texture(
        loaded_resources: &mut ResourceManager,
        context: &Context,
        id: TextureID,
    ) -> Arc<ImageView> {
        match loaded_resources.loaded_textures.get(&id) {
            Some(tex) => tex.clone(),
            None => {
                let path = match id {
                    TextureID::InaBody => "models/ina/Body_Base_Color.png",
                    TextureID::InaCloth => "models/ina/Cloth_Base_Color.png",
                    TextureID::InaHair => "models/ina/Hair_Base_Color.png",
                    TextureID::InaHead => "models/ina/Head_Base_Color.png",
                    TextureID::LostEmpire => "models/lost_empire-RGBA.png",
                };
                let tex = load_texture(&context.allocators, &context.queue, Path::new(path));
                loaded_resources.loaded_textures.insert(id, tex.clone());
                tex
            }
        }
    }
}

fn mesh_from_file<'a>(
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

/// creates a material of the given pipeline with a corresponding descriptor set as set 2
fn init_material(
    context: &Context,
    shader: &mut Shader,
    descriptor_writes: impl IntoIterator<Item = WriteDescriptorSet>,
) -> RenderSubmit {
    shader.add_material(Some(
        PersistentDescriptorSet::new(
            &context.allocators.descriptor_set,
            shader
                .pipeline
                .layout()
                .set_layouts()
                .get(2)
                .unwrap()
                .clone(),
            descriptor_writes,
            [],
        )
        .unwrap(), // pipeline_group.create_material_set(&self.context.allocators, descriptor_writes),
    ))
}
fn create_material_buffer<T: vulkano::buffer::BufferContents>(
    context: &Context,
    data: T,
    usage: vulkano::buffer::BufferUsage,
) -> vulkano::buffer::Subbuffer<T> {
    crate::vulkano_objects::buffers::create_material_buffer(&context.allocators, data, usage)
}
