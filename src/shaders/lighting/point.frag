// modified from vulkano defered lighting example

#version 450

// The `color_input` parameter of the `draw` method.
layout(input_attachment_index = 0, set = 0, binding = 0) uniform subpassInput u_diffuse;
// The `normals_input` parameter of the `draw` method.
layout(input_attachment_index = 1, set = 0, binding = 1) uniform subpassInput u_normals;
// The `depth_input` parameter of the `draw` method.
layout(input_attachment_index = 2, set = 0, binding = 2) uniform subpassInput u_depth;

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

layout(location = 0) in vec2 v_screen_coords;
layout(location = 1) in flat uint v_light_index;
layout(location = 0) out vec4 f_color;

void main() {
    // Any depth superior or equal to 1.0 means that the pixel has been untouched by 
    // the deferred pass. We don't want to deal with them.
    float in_depth = subpassLoad(u_depth).x;
    if (in_depth >= 1.0) {
        discard;
    }

    PointLight light = point_buffer.lights[v_light_index];

    // Find the world coordinates of the current pixel.
    vec4 world = scene_data.inv_view_proj * vec4(v_screen_coords, in_depth, 1.0);// just use gl_FragCoord? (no gl_FragCoord is in pixels)
    world /= world.w;
    vec3 light_displacement = world.xyz - light.position.xyz;

    // Calculate the percent of lighting that is received based on the orientation of 
    // the normal and the direction of the light.
    vec3 in_normal = normalize(subpassLoad(u_normals).rgb);
    vec3 light_direction = normalize(light_displacement);
    float light_percent = max(-dot(light_direction, in_normal), 0.0);

    // Further decrease light_percent based on the distance with the light position.
    float light_distance = dot(light_displacement, light_displacement);
    light_percent *= 1.0 / (light_distance + 1.0);

    // if (light_distance < 0.1) {
    //     f_color = vec4(1.0);
    // } else {
    vec3 in_diffuse = subpassLoad(u_diffuse).rgb;
    f_color = vec4(light.color.rgb * light_percent * in_diffuse, 1.0);
    
    // f_color = vec4(1.0,0.0,0.0,1.0);
}