// stolen from vulkano defered lighting example

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
    // billboard shenanigans
    vec3 offset_right = position.x * vec3(global_data.view[0][0], global_data.view[1][0], global_data.view[2][0]);
    vec3 offset_up = position.y * vec3(global_data.view[0][1], global_data.view[1][1], global_data.view[2][1]);
    // vec3 offset_forward = position.z * vec3(global_data.view[0][2], global_data.view[1][2], global_data.view[2][2]);

    GPUColoredData object = objectBuffer.objects[gl_InstanceIndex];
    vec4 object_position = object.render_matrix[3];

    // float light_radius = 4.0;
    float light_radius = 4.0;
    vec4 world_pos = vec4(object_position.xyz / object_position.w + light_radius * (offset_right + offset_up), 1.0);
    float view_pos_z = dot(-world_pos, vec4(global_data.view[0][2], global_data.view[1][2], global_data.view[2][2], global_data.view[3][2]));

    vec4 screen_pos = global_data.view_proj * world_pos;
    screen_pos /= screen_pos.w;
    screen_pos.z = sign(view_pos_z + light_radius) - 1.0;

    gl_Position = screen_pos;
    v_tex_coord = uv;
    v_normal = vec3(0.0, 0.0, 1.0);
    v_object_index = gl_InstanceIndex;
    // v_screen_coords = gl_Position.xy;
}