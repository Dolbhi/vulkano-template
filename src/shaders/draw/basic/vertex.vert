#version 460

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;
layout(location = 2) in vec3 colour;
layout(location = 3) in vec2 uv;

layout(set = 0, binding = 0) uniform GPUGlobalData {
    mat4 view;
    mat4 proj;
    mat4 view_proj;
    mat4 inv_view_proj;
} global_data;
struct GPUObjectData {
	mat4 render_matrix;
    mat4 normal_matrix;
};
layout(set = 1, binding = 0) readonly buffer ObjectBuffer {
    GPUObjectData objects[];
} objectBuffer;

layout(location = 0) out vec2 v_tex_coord;
layout(location = 1) out vec3 v_normal;

void main() {
    GPUObjectData objectData = objectBuffer.objects[gl_BaseInstance];
    
    gl_Position = global_data.view_proj * objectData.render_matrix * vec4(position, 1.0);
    v_tex_coord = uv;
    v_normal = normalize(mat3(objectData.normal_matrix) * normal);
}
