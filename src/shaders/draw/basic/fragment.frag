#version 460

layout(location = 0) in vec3 in_color;
layout(location = 1) in vec2 texCoord;

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
	vec4 dummy = global_data.ambient_color;

	vec4 tex_color = texture(s, texCoord);
	if (tex_color.a < 0.05) discard;

	f_color = tex_color;// pow(tex_color, vec4(1/2.2));
}