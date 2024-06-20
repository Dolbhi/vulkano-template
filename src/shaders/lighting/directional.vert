// stolen from vulkano defered lighting example

#version 460
#include "../includes/global_data.glsl"

layout(location = 0) in vec2 position;
layout(location = 0) out vec2 v_screen_coords;

layout(location = 1) out uint v_light_index;

void main() {
    // touch global data to include it
    float a = global_data.view[0][0];

    v_screen_coords = position;
    gl_Position = vec4(position, 0.0, 1.0);
    v_light_index = gl_InstanceIndex;
}