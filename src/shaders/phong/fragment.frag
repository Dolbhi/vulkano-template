#version 460

layout(location = 0) in vec2 texCoord;
layout(location = 1) in vec3 worldPos;
layout(location = 2) in vec3 normal;

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
	vec4 tex_color = texture(s, texCoord);
	if (tex_color.a < 0.05) discard;

	vec3 ambient = scene_data.ambient_color.xyz * tex_color.xyz;
	float diffuse_factor = dot(normalize(normal), normalize(vec3(-scene_data.sunlight_direction)));
	vec3 diffuse = max(diffuse_factor, 0) * vec3(tex_color) * vec3(scene_data.sunlight_color);

	f_color = vec4(ambient + diffuse, 1.0);
}