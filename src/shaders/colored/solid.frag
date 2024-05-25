#version 460

layout(location = 0) in vec2 v_tex_coord;
layout(location = 1) in vec3 v_normal;
// layout(location = 2) in flat uint v_object_index;

layout(set = 2, binding = 0) uniform SolidData {
    vec4 color;
} data;

// struct GPUObjectData {
// 	mat4 render_matrix;
//     mat4 normal_matrix;
//     // vec4 color;
// };
// layout(set = 1, binding = 0) readonly buffer ColoredBuffer {
//     GPUObjectData objects[];
// } objectBuffer;

layout(location = 0) out vec4 f_color;
layout(location = 1) out vec4 f_normal;

void main()
{
	vec4 color = data.color;// objectBuffer.objects[v_object_index].color;
	if (color.a < 0.05) discard;

	f_color = color;// pow(tex_color, vec4(1/2.2));
	f_normal = vec4(v_normal, 0.0);
}