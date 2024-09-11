#version 460
#include "../includes/global_data.glsl"
#include "../includes/aabb.glsl"

layout(location = 0) in vec3 position;

layout(set = 1, binding = 0) readonly buffer BoxBuffer {
    GPUAABB boxes[];
} box_buffer;

layout(location = 0) out vec2 v_tex_coord;
layout(location = 1) out vec3 v_normal;
layout(location = 2) out uint v_object_index;

void main() {
    GPUAABB box_data = box_buffer.boxes[gl_InstanceIndex];
    
    gl_Position = global_data.view_proj * vec4(position * (box_data.max - box_data.min) + box_data.min, 1.0);
    v_tex_coord = vec2(0, 0);
    v_normal = vec3(0, 0, 0);
    v_object_index = gl_InstanceIndex;
}
