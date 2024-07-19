// mod frame_data;
// use frame_data::FrameData;

use std::collections::BTreeMap;

use vulkano::{
    command_buffer::AutoCommandBufferBuilder,
    descriptor_set::DescriptorSetsCollection,
    pipeline::{
        graphics::{
            vertex_input::{Vertex, VertexDefinition},
            GraphicsPipelineCreateInfo,
        },
        PipelineShaderStageCreateInfo,
    },
    render_pass::Subpass,
};

use crate::{
    render::{context::Context, render_data::material::Shader},
    vulkano_objects::pipeline::{
        window_size_dependent_pipeline_info, LayoutOverrides, PipelineHandler,
    },
    VertexFull,
};

/// Collection of shaders, meant to be run on a single subpass
///
/// K: ID type of shaders
///
/// T: See [Shader<T>]
///
/// All shader pipelines share sets 0 and 1, describing global scene data and an array of object data (storage buffer) respectively
///
/// Materials can optionally add more sets, starting from set 2
///
/// Should always have at least one shader present
pub struct DrawSystem<K: Ord, T: Clone> {
    pub shaders: BTreeMap<K, Shader<T>>,
    layout_overrides: LayoutOverrides,
    // layout: PipelineLayout,
    // subpass: Subpass,
}

impl<K: Ord, T: Clone> DrawSystem<K, T> {
    /// Creates `DrawSystem` from the stage create infos of a starting shader
    pub fn new(
        context: &Context,
        subpass: &Subpass,
        id: K,
        stages: [PipelineShaderStageCreateInfo; 2],
        // layout: Arc<PipelineLayout>,
        layout_overrides: LayoutOverrides,
    ) -> Self {
        let vertex_input_state = VertexFull::per_vertex()
            .definition(&stages[0].entry_point.info().input_interface) //[Position::per_vertex(), Normal::per_vertex()]
            .unwrap();

        let layout = layout_overrides.create_layout(context.device.clone(), &stages);

        let shader = Shader::new(PipelineHandler::new(
            context.device.clone(),
            window_size_dependent_pipeline_info(
                stages,
                layout,
                vertex_input_state,
                context.viewport.clone(),
                subpass.clone(),
                crate::vulkano_objects::pipeline::PipelineType::Drawing,
            ),
        ));

        // let layouts = shader.pipeline.layout().set_layouts();
        // let layouts = [layouts[0].clone(), layouts[1].clone()];

        DrawSystem {
            shaders: BTreeMap::from([(id, shader)]),
            layout_overrides, // subpass: subpass.clone(),
        }
    }

    /// Creates shader with the same subpass and dynamic bindings as this system
    ///
    /// New shaders will make use of the `GraphicsPipelineCreateInfo` of the first shader with layout and stages substituted
    pub fn add_shader(
        &mut self,
        context: &Context,
        id: impl Into<K>,
        stages: [PipelineShaderStageCreateInfo; 2],
    ) {
        let layout = self
            .layout_overrides
            .create_layout(context.device.clone(), &stages);

        let create_info = GraphicsPipelineCreateInfo {
            layout,
            stages: stages.into_iter().collect(),
            ..self.first_shader().pipeline.create_info.clone()
        };

        let pipeline = PipelineHandler::new(context.device.clone(), create_info);

        self.shaders.insert(id.into(), Shader::new(pipeline));
    }

    pub fn first_shader(&self) -> &Shader<T> {
        self.shaders.first_key_value().unwrap().1
    }

    /// search for shader via some ID
    pub fn find_shader<'a>(&'a mut self, id: impl Into<&'a K>) -> Option<&'a mut Shader<T>> {
        self.shaders.get_mut(id.into())
    }

    /// Recreate all pipelines with any changes in viewport
    ///
    /// See also: [recreate_pipeline](PipelineHandler::recreate_pipeline)
    pub fn recreate_pipelines(&mut self, context: &Context) {
        for pipeline in self.shaders.values_mut() {
            pipeline
                .pipeline
                .recreate_pipeline(context.device.clone(), context.viewport.clone());
        }
    }

    // /// sort and write object data to given storage buffer (must be called before rendering)
    // pub fn update_object_buffer<O: BufferContents + From<Matrix4<f32>>>(
    //     &mut self,
    //     buffer: &Subbuffer<[O]>,
    //     offset: usize,
    // ) -> Option<usize> {
    //     let obj_iter = self
    //         .shaders
    //         .iter_mut()
    //         .flat_map(|shader| shader.upload_pending_objects());
    //     write_to_storage_buffer(buffer, obj_iter, offset)
    // }

    /// bind draw calls to the given command buffer builder, be sure to call `update_object_buffer()` before hand
    pub fn render<P, A: vulkano::command_buffer::allocator::CommandBufferAllocator>(
        &mut self,
        object_index: &mut u32,
        sets: impl DescriptorSetsCollection + Clone,
        command_builder: &mut AutoCommandBufferBuilder<P, A>,
    ) {
        for pipeline_group in self.shaders.values_mut() {
            pipeline_group.draw_objects(object_index, sets.clone(), command_builder);
        }
    }

    // fn clear_unused_resource(&mut self) {
    //     for pipeline_group in self.pipelines.iter() {
    //         for material in pipeline_group.materials.iter() {
    //             if self.pending_objects
    //         }
    //     }
    // }
}
