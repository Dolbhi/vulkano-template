#version 460

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;
layout(location = 2) in vec3 colour;

layout(set = 0, binding = 0) uniform Data {
    // vec3 color;
    vec2 position;
} uniforms;

layout(location = 0) out vec3 outColor;

// consts
layout( push_constant ) uniform constants{
	vec4 data;
	mat4 render_matrix;
} PushConstants;

void main() {
    outColor = colour;
    vec4 pos = vec4(
        position.x + uniforms.position.x, 
        position.y + uniforms.position.y, 
        position.z, 
        1.0
    );
    gl_Position = PushConstants.render_matrix * pos; 
}
