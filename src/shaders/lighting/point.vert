// stolen from vulkano defered lighting example

#version 460
#include "../includes/global_data.glsl"

layout(location = 0) in vec2 position;
layout(location = 0) out vec2 v_screen_coords;

struct PointLight {
    // The `color` parameter of the `draw` method.
    vec4 color;
    // The `position` parameter of the `draw` method.
    vec4 position;
};
layout(set = 2, binding = 0) readonly buffer PointLights {
    PointLight lights[];
} point_buffer;

layout(location = 1) out uint v_light_index;

void main() {
    // billboard shenanigans
    vec3 offset_right = position.x * vec3(global_data.view[0][0],global_data.view[1][0],global_data.view[2][0]);
    vec3 offset_up = position.y * vec3(global_data.view[0][1],global_data.view[1][1],global_data.view[2][1]);

    PointLight light = point_buffer.lights[gl_InstanceIndex];
    float light_radius = light.position.w;
    vec4 world_pos = vec4(light.position.xyz + light_radius * (offset_right + offset_up), 1.0);
    float view_pos_z = dot(-world_pos, vec4(global_data.view[0][2],global_data.view[1][2],global_data.view[2][2],global_data.view[3][2]));

    vec4 screen_pos = global_data.view_proj * world_pos;
    screen_pos /= screen_pos.w;
    screen_pos.z = sign(view_pos_z + light_radius) - 1.0;

    gl_Position = screen_pos;
    v_screen_coords = gl_Position.xy;
    v_light_index = gl_InstanceIndex;
}