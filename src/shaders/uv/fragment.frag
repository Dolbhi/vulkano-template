#version 460

layout(location = 0) in vec3 color;
layout(location = 1) in vec2 texCoord;

layout(set = 0, binding = 1) uniform GPUSceneData {
    vec4 fog_color; 			// w is for exponent
	vec4 fog_distances; 		// x for min, y for max, zw unused.
	vec4 ambient_color;
	vec4 sunlight_direction; 	// w for sun power
	vec4 sunlight_color;
} sceneData;

layout(location = 0) out vec4 f_color;

void main()
{
	f_color = vec4(texCoord * sceneData.ambient_color.xy, 0.0f, 1.0f);

    // float step_alpha = floor(tex_color.a * 10) / 10.f;

	f_color = vec4(texCoord, 0.0f, 1.0f);
}