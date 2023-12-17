// modified from vulkano defered lighting example

#version 450

// The `color_input` parameter of the `draw` method.
layout(input_attachment_index = 0, set = 0, binding = 0) uniform subpassInput u_diffuse;
// The `normals_input` parameter of the `draw` method.
layout(input_attachment_index = 1, set = 0, binding = 1) uniform subpassInput u_normals;
// The `depth_input` parameter of the `draw` method.
layout(input_attachment_index = 2, set = 0, binding = 2) uniform subpassInput u_depth;

layout(set = 1, binding = 0) uniform GPULightingData {
    // The `screen_to_world` parameter of the `draw` method.
    mat4 screen_to_world;
    vec4 ambient_color;
    uint point_light_count;
    uint direction_light_count;
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

struct DirectionLight {
    // The `color` parameter of the `draw` method.
    vec4 color;
    // The `direction` parameter of the `draw` method.
    vec4 direction;
};
layout(set = 3, binding = 0) readonly buffer DirectionLights {
    DirectionLight lights[];
} direction_buffer;

layout(location = 0) in vec2 v_screen_coords;
layout(location = 0) out vec4 f_color;

vec3 point_lighting(PointLight light, vec4 world, vec3 in_normal) {
    vec3 light_direction = normalize(light.position.xyz - world.xyz);

    // Calculate the percent of lighting that is received based on the orientation of 
    // the normal and the direction of the light.
    float light_percent = max(-dot(light_direction, in_normal), 0.0);

    float light_distance = length(light.position.xyz - world.xyz);
    // Further decrease light_percent based on the distance with the light position.
    light_percent *= 1.0 / exp(light_distance);

    return light.color.rgb * light_percent;
}

vec3 direction_lighting(DirectionLight light, vec3 in_normal) {
    // If the normal is perpendicular to the direction of the lighting, then 
    // `light_percent` will be 0. If the normal is parallel to the direction of the 
    // lightin, then `light_percent` will be 1. Any other angle will yield an 
    // intermediate value.
    float light_percent = max(-dot(light.direction.xyz, in_normal), 0.0);

    return light_percent * light.color.rgb;
}

void main() {
    float in_depth = subpassLoad(u_depth).x;
    // Any depth superior or equal to 1.0 means that the pixel has been untouched by 
    // the deferred pass. We don't want to deal with them.
    if (in_depth >= 1.0) {
        discard;
    }
    vec3 in_normal = normalize(subpassLoad(u_normals).rgb);
    vec3 in_diffuse = subpassLoad(u_diffuse).rgb;

    // Find the world coordinates of the current pixel.
    vec4 world = scene_data.screen_to_world * vec4(v_screen_coords, in_depth, 1.0);// just use gl_FragCoord?
    world /= world.w;

    f_color = vec4(0.0, 0.0, 0.0, 1.0);
    // point lights
    for (int i = 0; i < scene_data.point_light_count; i++) {
        f_color.rgb += point_lighting(point_buffer.lights[i], world, in_normal) * in_diffuse;
    }
    // directional lights
    for (int i = 0; i < scene_data.direction_light_count; i++) {
        f_color.rgb += direction_lighting(direction_buffer.lights[i], in_normal) * in_diffuse;
    }
    // ambient lights
    f_color.rgb += scene_data.ambient_color.rgb * in_diffuse;
}