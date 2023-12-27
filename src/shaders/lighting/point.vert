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

layout(location = 0) in vec2 offset;
layout(location = 0) out vec2 v_screen_coords;
layout(location = 1) out uint v_light_index;

void main() {
    vec3 offset_right = offset.x * vec3(scene_data.view[0][0],scene_data.view[1][0],scene_data.view[2][0]);
    vec3 offset_up = offset.y * vec3(scene_data.view[0][1],scene_data.view[1][1],scene_data.view[2][1]);

    float light_radius = 1.0;
    vec3 world_pos = point_buffer.lights[gl_BaseInstance].position.xyz + light_radius * (offset_right + offset_up);

    gl_Position = scene_data.view_proj * vec4(world_pos, 1.0);
    v_screen_coords = gl_Position.xy;
    v_light_index = gl_BaseInstance;
}