#version 460

layout(location = 0) in vec2 texCoord;
layout(location = 1) in vec3 worldPos;
layout(location = 2) in vec3 normal;

layout(set = 0, binding = 0) uniform GPUGlobalData {
    mat4 view;
    mat4 proj;
    mat4 view_proj;
    vec4 ambient_color;
	vec4 sunlight_direction; 	// w for sun power
	vec4 sunlight_color;
} global_data;
layout(set = 2, binding = 0) uniform sampler2D s;

layout(location = 0) out vec4 f_color;

void main()
{
	vec4 tex_color = texture(s, texCoord);
	if (tex_color.a < 0.05) discard;

	vec3 ambient = global_data.ambient_color.xyz * tex_color.xyz;
	float diffuse_factor = dot(normalize(normal), normalize(vec3(-global_data.sunlight_direction)));
	vec3 diffuse = max(diffuse_factor, 0) * vec3(tex_color) * vec3(global_data.sunlight_color);

	f_color = vec4(ambient + diffuse, 1.0);
}