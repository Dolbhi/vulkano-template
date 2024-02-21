#version 460

layout(location = 0) in vec2 v_tex_coord;
layout(location = 1) in vec3 v_normal;

layout(location = 0) out vec4 f_color;
layout(location = 1) out vec4 f_normal;

void main()
{
    float step = ceil(v_tex_coord.x * 10) / 10;
    step = pow(step, 1/2.2);
	f_color = vec4(vec3(step), 1.0);
	f_normal = vec4(v_normal, 0.0);
}