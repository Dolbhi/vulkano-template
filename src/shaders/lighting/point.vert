// stolen from vulkano defered lighting example

#version 460

layout(set = 1, binding = 0) uniform GPUGlobalData {
    mat4 view;
    mat4 proj;
    mat4 view_proj;
    mat4 inv_view_proj;
} scene_data;

struct PointLight {
    // The `color` parameter of the `draw` method.
    vec4 color;
    // The `position` parameter of the `draw` method.
    vec4 position;
};
layout(set = 2, binding = 0) readonly buffer PointLights {
    PointLight lights[];
} point_buffer;

layout(location = 0) in vec2 position;
layout(location = 0) out vec2 v_screen_coords;
layout(location = 1) out uint v_light_index;

void main() {
    vec3 offset_right = position.x * vec3(scene_data.view[0][0],scene_data.view[1][0],scene_data.view[2][0]);
    vec3 offset_up = position.y * vec3(scene_data.view[0][1],scene_data.view[1][1],scene_data.view[2][1]);

    float light_radius = 2.0;
    vec4 world_pos = vec4(point_buffer.lights[gl_BaseInstance].position.xyz + light_radius * (offset_right + offset_up), 1.0);
    float view_pos_z = dot(-world_pos, vec4(scene_data.view[0][2],scene_data.view[1][2],scene_data.view[2][2],scene_data.view[3][2]));

    vec4 screen_pos = scene_data.view_proj * world_pos;
    screen_pos /= screen_pos.w;
    screen_pos.z = sign(view_pos_z + light_radius) - 1.0;

    gl_Position = screen_pos;
    v_screen_coords = gl_Position.xy;
    v_light_index = gl_BaseInstance;
}