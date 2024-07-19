/// Rendering Heirachy:
///
/// System < Shader < Material < Object
///
/// Limitations:
/// 1. Shader and Material must be efficiently searched for
/// 2. Some shaders require specific material sets
pub mod material;
pub mod mesh;
pub mod render_object;
pub mod texture;
