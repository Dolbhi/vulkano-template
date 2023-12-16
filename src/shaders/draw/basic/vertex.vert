#version 460

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;
layout(location = 2) in vec3 colour;
layout(location = 3) in vec2 uv;

layout(set = 0, binding = 0) uniform GPUGlobalData {
    mat4 view;
    mat4 proj;
    mat4 view_proj;
    vec4 ambient_color;
	vec4 sunlight_direction; 	// w for sun power
	vec4 sunlight_color;
} global_data;
struct GPUObjectData {
	mat4 render_matrix;
    mat4 normal_matrix;
};
layout(set = 1, binding = 0) readonly buffer ObjectBuffer {
    GPUObjectData objects[];
} objectBuffer;

layout(location = 0) out vec3 outColor;
layout(location = 1) out vec2 outTexCoord;

void main() {
    vec4 localPos = vec4(position, 1.0);
    localPos = global_data.view_proj * objectBuffer.objects[gl_BaseInstance].render_matrix * localPos;
    
    gl_Position = localPos;
    outColor = colour;
    outTexCoord = uv;
}
