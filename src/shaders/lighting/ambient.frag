// modified from vulkano defered lighting example

#version 450

layout(set = 0, binding = 0) uniform GPUGlobalData {
    mat4 view;
    mat4 proj;
    mat4 view_proj;
    mat4 inv_view_proj;
} scene_data;

// The `color_input` parameter of the `draw` method.
layout(input_attachment_index = 0, set = 1, binding = 0) uniform subpassInput u_diffuse;
// The `normals_input` parameter of the `draw` method.
layout(input_attachment_index = 1, set = 1, binding = 1) uniform subpassInput u_normals;
// The `depth_input` parameter of the `draw` method.
layout(input_attachment_index = 2, set = 1, binding = 2) uniform subpassInput u_depth;

layout(push_constant) uniform GPUAmbientData {
    vec4 ambient_color;
};

layout(location = 0) in vec2 v_screen_coords;
layout(location = 1) in flat uint v_light_index;
layout(location = 0) out vec4 f_color;

void main() {
    // touch global data to include it
    float a = scene_data.view[0][0];

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