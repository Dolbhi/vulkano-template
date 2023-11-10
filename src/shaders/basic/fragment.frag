#version 460

layout(location = 0) in vec3 color;

layout(location = 0) out vec4 f_color;

layout(set = 0, binding = 1) uniform GPUSceneData {
    vec4 fog_color; 			// w is for exponent
	vec4 fog_distances; 		// x for min, y for max, zw unused.
	vec4 ambient_color;
	vec4 sunlight_direction; 	// w for sun power
	vec4 sunlight_color;
} sceneData;

void main()
{
	f_color = vec4(color + sceneData.ambient_color.xyz,1.0f);
}