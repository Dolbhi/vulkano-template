#version 460

layout(location = 0) in vec3 in_color;
layout(location = 1) in vec2 tex_coord;

layout(set = 2, binding = 0) uniform sampler2D s;

layout(location = 0) out vec4 f_color;
layout(location = 1) out vec4 f_normal;

void main()
{
	vec4 tex_color = texture(s, tex_coord);

    // float step_alpha = floor(tex_color.a * 10) / 10.f;

	f_color = vec4(tex_color.a);
    f_normal = vec4(0.0, 0.0, 1.0, 0.0);
}