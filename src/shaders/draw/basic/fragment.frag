#version 460

layout(location = 0) in vec2 v_tex_coord;
layout(location = 1) in vec3 v_normal;

layout(set = 2, binding = 0) uniform sampler2D s;

layout(location = 0) out vec4 f_color;
layout(location = 1) out vec4 f_normal;

void main()
{
	vec4 tex_color = texture(s, v_tex_coord);
	if (tex_color.a < 0.05) discard;

	f_color = tex_color;// pow(tex_color, vec4(1/2.2));
	f_normal = vec4(v_normal, 0.0);
}