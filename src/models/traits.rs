use vulkano::buffer::BufferContents;

pub trait Model<V: BufferContents, U: BufferContents> {
    fn get_indices() -> Vec<u32>;
    fn get_vertices() -> Vec<V>;
    fn get_initial_uniform_data() -> U;
}
