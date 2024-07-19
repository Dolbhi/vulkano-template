#version 460
#include "../includes/draw_vert_in.glsl"
#include "../includes/global_data.glsl"
#include "../includes/colored_data.glsl"

layout(set = 1, binding = 0) readonly buffer ColoredBuffer {
    GPUColoredData objects[];
} objectBuffer;

layout(location = 0) out vec2 v_tex_coord;
layout(location = 1) out vec3 v_normal;
layout(location = 2) out uint v_object_index;

void main() {
    GPUColoredData objectData = objectBuffer.objects[gl_InstanceIndex];
    
    gl_Position = global_data.view_proj * objectData.render_matrix * vec4(position, 1.0);
    v_tex_coord = uv;
    v_normal = normalize(mat3(objectData.normal_matrix) * normal);
    v_object_index = gl_InstanceIndex;
}
