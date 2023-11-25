#version 460

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;
layout(location = 2) in vec3 colour;
layout(location = 3) in vec2 uv;

layout(set = 0, binding = 0) uniform GPUCameraData {
    mat4 view;
    mat4 proj;
    mat4 view_proj;
} cameraData;
struct GPUObjectData {
    // vec3 color;
	vec4 data;
	mat4 render_matrix;
    mat4 normal_matrix;
};
layout(set = 1, binding = 0) readonly buffer ObjectBuffer {
    GPUObjectData objects[];
} objectBuffer;

layout(location = 0) out vec2 outTexCoord;
layout(location = 1) out vec3 outWorldPos;
layout(location = 2) out vec3 outNormal;

void main() {
    GPUObjectData objectData = objectBuffer.objects[gl_BaseInstance];

    vec4 worldPos = objectData.render_matrix * vec4(position, 1.0);
    
    gl_Position = cameraData.view_proj * worldPos;
    outWorldPos = worldPos.xyz;
    outTexCoord = uv;
    outNormal = normalize(mat3(objectData.normal_matrix) * normal);
}
