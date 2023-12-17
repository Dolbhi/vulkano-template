#version 460

layout(location = 0) in vec2 v_tex_coord;
layout(location = 1) in vec3 v_normal;

layout(set = 0, binding = 0) uniform GPUGlobalData {
    mat4 view;
    mat4 proj;
    mat4 view_proj;
    vec4 ambient_color;
	vec4 sunlight_direction; 	// w for sun power
	vec4 sunlight_color;
} global_data;

layout(location = 0) out vec4 f_color;
layout(location = 1) out vec4 f_normal;

void main()
{
    // float step_alpha = floor(tex_color.a * 10) / 10.f;

	f_color = vec4(v_tex_coord, 0.0, 1.0);// vec4(v_tex_coord, 0.0f, 1.0f);
    f_normal = vec4(v_normal, 0.0);
}