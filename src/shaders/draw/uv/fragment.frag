#version 460
// #include "../../includes/global_data.glsl"

layout(location = 0) in vec2 v_tex_coord;
layout(location = 1) in vec3 v_normal;

layout(location = 0) out vec4 f_color;
layout(location = 1) out vec4 f_normal;

void main()
{
    // float step_alpha = floor(tex_color.a * 10) / 10.f;

	f_color = vec4(v_tex_coord, 0.0, 1.0);
    f_normal = vec4(v_normal, 0.0);
}