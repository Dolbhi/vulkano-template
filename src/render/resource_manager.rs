use std::{collections::HashMap, iter::zip, path::Path, sync::Arc};

use cgmath::Matrix4;
use vulkano::{
    buffer::Subbuffer,
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    image::{sampler::Sampler, view::ImageView},
};

use crate::{
    shaders::{self, SolidData},
    vulkano_objects::buffers::MeshBuffers,
    VertexFull,
};

use super::{
    mesh::from_obj,
    render_data::{
        material::Shader,
        texture::{create_sampler, load_texture},
    },
    renderer::systems::DrawSystem,
    Context, RenderSubmit,
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

/// Unique ID to identify materials, descriminant corresponds to shader
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum MaterialID {
    Texture(TextureID),
    Color(u32),
    UV,
    Gradient,
    Billboard,
    // Parameter(u32),
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum TextureID {
    InaBody,
    InaCloth,
    InaHair,
    InaHead,
    LostEmpire,
}

/// Stores currently loaded resources of the renderer
///
/// Call `begin_retrieving` to retrieve resources
pub struct ResourceManager {
    loaded_meshes: HashMap<MeshID, Arc<MeshBuffers<VertexFull>>>,
    loaded_materials: HashMap<(MaterialID, bool), RenderSubmit<Matrix4<f32>>>,
    loaded_textures: HashMap<TextureID, Arc<ImageView>>,
    linear_sampler: Arc<Sampler>,
    next_color_id: u32,
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
            next_color_id: 0,
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
    pub fn get_mesh(&mut self, id: MeshID) -> Arc<MeshBuffers<VertexFull>> {
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
                        let mesh = Arc::new(MeshBuffers::initialize_device_local(
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
                            mesh_from_file(self.context, "models/ina/ReadyToRigINA.obj").skip(2),
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
                            mesh_from_file(self.context, "models/lost_empire.obj").enumerate()
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
                            .insert(id, mesh_from_file(self.context, path).next().unwrap());
                    }
                };
                // try fetch again
                loaded_meshes[&id].clone()
            }
        }
    }

    pub fn get_material(&mut self, id: MaterialID, lit: bool) -> RenderSubmit<Matrix4<f32>> {
        match self.loaded_resources.loaded_materials.get(&(id, lit)) {
            Some(mat) => mat.clone(),
            None => {
                // Narrow down system
                let system = if lit {
                    &mut self.lit_system
                } else {
                    &mut self.unlit_system
                };

                // Narrow down shader
                let shader = match system.find_shader(id) {
                    Some(s) => s,
                    None => {
                        {
                            // load shader
                            match id {
                                MaterialID::Texture(_) => {
                                    panic!("Texture shader should be loaded by default")
                                }
                                MaterialID::Color(_) => {
                                    panic!("Color shader should be loaded when creating solid material")
                                }
                                MaterialID::UV => {
                                    system.add_shader(
                                        self.context,
                                        id,
                                        shaders::load_basic_vs(self.context.device.clone())
                                            .expect("failed to create uv shader module"),
                                        shaders::load_uv_fs(self.context.device.clone())
                                            .expect("failed to create uv shader module"),
                                    );
                                }
                                MaterialID::Gradient => {
                                    system.add_shader(
                                        self.context,
                                        id,
                                        shaders::load_basic_vs(self.context.device.clone())
                                            .expect("failed to create grad shader module"),
                                        shaders::load_grad_fs(self.context.device.clone())
                                            .expect("failed to create grad shader module"),
                                    );
                                }
                                MaterialID::Billboard => system.add_shader(
                                    self.context,
                                    id,
                                    shaders::load_billboard_vs(self.context.device.clone())
                                        .expect("failed to create billboard shader module"),
                                    shaders::load_solid_fs(self.context.device.clone())
                                        .expect("failed to create billboard shader module"),
                                ),
                            };
                            system.find_shader(id).unwrap()
                        }
                    }
                };
                // make material
                let material = match id {
                    MaterialID::Texture(tex_id) => {
                        let tex = Self::get_texture(self.loaded_resources, self.context, tex_id);
                        init_material(
                            self.context,
                            shader,
                            [WriteDescriptorSet::image_view_sampler(
                                0,
                                tex,
                                self.loaded_resources.linear_sampler.clone(),
                            )],
                        )
                    }
                    MaterialID::Color(_) => {
                        panic!("Solid material not found, it must be loaded with load")
                        // let color_buffer = create_material_buffer(
                        //     self.context,
                        //     draw::SolidData {
                        //         color: color.map(|v| (v as f32) / (u8::MAX as f32)),
                        //     },
                        //     vulkano::buffer::BufferUsage::empty(),
                        // );
                        // init_material(
                        //     self.context,
                        //     shader,
                        //     [WriteDescriptorSet::buffer(0, color_buffer)],
                        // )
                    }
                    MaterialID::Billboard => {
                        let color_buffer = create_material_buffer(
                            self.context,
                            shaders::SolidData {
                                color: [1.0, 0.0, 1.0, 1.0],
                            },
                            vulkano::buffer::BufferUsage::empty(),
                        );
                        init_material(
                            self.context,
                            shader,
                            [WriteDescriptorSet::buffer(0, color_buffer)],
                        )
                    }
                    _ => shader.add_material(None),
                };
                self.loaded_resources
                    .loaded_materials
                    .insert((id, lit), material.clone());
                material
            }
        }
    }

    /// Create a material with a solid color, returns the material id, the subbuffer holding the color and the rendersubmit arc mutex
    pub fn load_solid_material(
        &mut self,
        color: [f32; 4],
        lit: bool,
    ) -> (MaterialID, Subbuffer<SolidData>, RenderSubmit<Matrix4<f32>>) {
        // Narrow down system
        let system = if lit {
            &mut self.lit_system
        } else {
            &mut self.unlit_system
        };

        if let None = system.find_shader(MaterialID::Color(0)) {
            system.add_shader(
                self.context,
                MaterialID::Color(0),
                shaders::load_basic_vs(self.context.device.clone())
                    .expect("failed to create solid shader module"),
                shaders::load_solid_fs(self.context.device.clone())
                    .expect("failed to create solid shader module"),
            );
        }
        let shader = system.find_shader(MaterialID::Color(0)).unwrap();

        let id = MaterialID::Color(self.loaded_resources.next_color_id);
        self.loaded_resources.next_color_id += 1;
        let color_buffer = create_material_buffer(
            self.context,
            shaders::SolidData { color },
            vulkano::buffer::BufferUsage::empty(),
        );
        let mat = init_material(
            self.context,
            shader,
            [WriteDescriptorSet::buffer(0, color_buffer.clone())],
        );

        self.loaded_resources
            .loaded_materials
            .insert((id, lit), mat.clone());

        (id, color_buffer, mat)
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
) -> impl Iterator<Item = Arc<MeshBuffers<VertexFull>>> + 'a {
    from_obj(Path::new(path))
        .into_iter()
        .map(|(vertices, indices)| {
            Arc::new(MeshBuffers::initialize_device_local(
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
) -> RenderSubmit<Matrix4<f32>> {
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
