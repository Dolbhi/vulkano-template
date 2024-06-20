// modified from vulkano defered lighting example

#version 450
#include "../includes/global_data.glsl"
#include "../includes/light_attachments.glsl"

layout(location = 0) in vec2 v_screen_coords;
layout(location = 1) in flat uint v_light_index;

layout(push_constant) uniform GPUAmbientData {
    vec4 ambient_color;
};

layout(location = 0) out vec4 f_color;

void main() {
    // touch global data to include it
    float a = global_data.view[0][0];

    // Any depth superior or equal to 1.0 means that the pixel has been untouched by 
    // the deferred pass. We don't want to deal with them.
    float in_depth = subpassLoad(u_depth).x;
    if (in_depth >= 1.0) {
        discard;
    }

    vec3 in_normal = normalize(subpassLoad(u_normals).rgb);

    vec3 in_diffuse = subpassLoad(u_diffuse).rgb;
    f_color = vec4(in_diffuse * ambient_color.xyz, 1.0);
}