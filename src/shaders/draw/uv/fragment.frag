#version 460

layout(location = 0) in vec3 color;
layout(location = 1) in vec2 texCoord;

layout(set = 0, binding = 0) uniform GPUGlobalData {
    mat4 view;
    mat4 proj;
    mat4 view_proj;
    vec4 ambient_color;
	vec4 sunlight_direction; 	// w for sun power
	vec4 sunlight_color;
} global_data;

layout(location = 0) out vec4 f_color;

void main()
{
	f_color = vec4(texCoord * global_data.ambient_color.xy, 0.0f, 1.0f);

    // float step_alpha = floor(tex_color.a * 10) / 10.f;

	f_color = vec4(texCoord, 0.0f, 1.0f);
}