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

struct DirectionLight {
    // The `color` parameter of the `draw` method, the w component corresponds to intensity
    vec4 color;
    // The `direction` parameter of the `draw` method.
    vec4 direction;
};
layout(set = 2, binding = 0) readonly buffer DirectionLights {
    DirectionLight lights[];
} direction_buffer;

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

    DirectionLight light = direction_buffer.lights[v_light_index];

    // If the normal is perpendicular to the direction of the lighting, then 
    // `light_percent` will be 0. If the normal is parallel to the direction of the 
    // lightin, then `light_percent` will be 1. Any other angle will yield an 
    // intermediate value.
    vec3 in_normal = normalize(subpassLoad(u_normals).rgb);
    float light_percent = max(-dot(light.direction.xyz, in_normal), 0.0);

    vec3 in_diffuse = subpassLoad(u_diffuse).rgb;
    f_color = vec4(light.color.w * light.color.rgb * light_percent * in_diffuse, 1.0);
}