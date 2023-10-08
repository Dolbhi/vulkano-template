#version 460

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;
layout(location = 2) in vec3 colour;

layout(set = 0, binding = 0) uniform Data {
    // vec3 color;
	vec4 data;
	mat4 render_matrix;
} uniforms;

layout(location = 0) out vec3 outColor;

void main() {
    outColor = colour;
    vec4 localPos = vec4(position, 1.0);
    localPos = uniforms.render_matrix * localPos;
    
    gl_Position = vec4(
        localPos.x,
        localPos.y, 
        localPos.z, 
        localPos.w
    );
}
