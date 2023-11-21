#version 460

layout(location = 0) in vec3 in_color;
layout(location = 1) in vec2 texCoord;


layout(set = 0, binding = 1) uniform GPUSceneData {
    vec4 fog_color; 			// w is for exponent
	vec4 fog_distances; 		// x for min, y for max, zw unused.
	vec4 ambient_color;
	vec4 sunlight_direction; 	// w for sun power
	vec4 sunlight_color;
} scene_data;

layout(set = 2, binding = 0) uniform sampler2D s;

layout(location = 0) out vec4 f_color;

void main()
{
	vec4 dummy = scene_data.ambient_color;

	f_color = pow(texture(s, texCoord), vec4(1/2.2));
}