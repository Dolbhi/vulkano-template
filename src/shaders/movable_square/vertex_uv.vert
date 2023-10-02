#version 460

layout(location = 0) in vec2 position;

layout(set = 0, binding = 0) uniform Data {
    vec3 color;
    vec2 position;
} uniforms;

layout(location = 0) out vec3 outColor;

void main() {
    //const array of colors for the triangle
	const vec3 colors[4] = vec3[4](
		vec3(0.0f, 0.0f, 0.0f), //black
		vec3(1.0f, 0.0f, 0.0f), //red
		vec3(0.0f, 1.0f, 0.0f), //green
		vec3(1.0f, 1.0f, 0.0f)  //yellow
	);

	outColor = colors[gl_VertexIndex];

    // outColor = uniforms.color;
    gl_Position = vec4(
        position.x + uniforms.position.x, 
        position.y + uniforms.position.y, 
        0.0, 
        1.0
    );
}
