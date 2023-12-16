// modified from vulkano defered lighting example

#version 450

// The `color_input` parameter of the `draw` method.
layout(input_attachment_index = 0, set = 0, binding = 0) uniform subpassInput u_diffuse;
// The `normals_input` parameter of the `draw` method.
layout(input_attachment_index = 1, set = 0, binding = 1) uniform subpassInput u_normals;
// The `depth_input` parameter of the `draw` method.
layout(input_attachment_index = 2, set = 0, binding = 2) uniform subpassInput u_depth;

layout(set = 0, binding = 0) uniform GPUSceneData {
    // The `screen_to_world` parameter of the `draw` method.
    mat4 screen_to_world;
    int point_light_count;
} scene_data;

struct Light {
    // The `color` parameter of the `draw` method.
    vec4 color;
    // The `position` parameter of the `draw` method.
    vec4 position;
};
layout(set = 1, binding = 0) readonly buffer Lights {
    Light lights[];
} object_buffer;

layout(location = 0) in vec2 v_screen_coords;
layout(location = 0) out vec4 f_color;

void main() {
    float in_depth = subpassLoad(u_depth).x;

    // Any depth superior or equal to 1.0 means that the pixel has been untouched by 
    // the deferred pass. We don't want to deal with them.
    if (in_depth >= 1.0) {
        discard;
    }

    // Find the world coordinates of the current pixel.
    vec4 world = scene_data.screen_to_world * vec4(v_screen_coords, in_depth, 1.0);
    world /= world.w;

    f_color = vec4(0.0, 0.0, 0.0, 1.0);

    for (int i = 0; i < scene_data.point_light_count; i++) {
        Light light = object_buffer.lights[i];

        vec3 in_normal = normalize(subpassLoad(u_normals).rgb);
        vec3 light_direction = normalize(light.position.xyz - world.xyz);

        // Calculate the percent of lighting that is received based on the orientation of 
        // the normal and the direction of the light.
        float light_percent = max(-dot(light_direction, in_normal), 0.0);

        float light_distance = length(light.position.xyz - world.xyz);
        // Further decrease light_percent based on the distance with the light position.
        light_percent *= 1.0 / exp(light_distance);

        vec3 in_diffuse = subpassLoad(u_diffuse).rgb;
        f_color.rgb += light.color.rgb * light_percent * in_diffuse;
    }
}