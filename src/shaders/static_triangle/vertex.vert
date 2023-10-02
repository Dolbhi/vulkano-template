#version 460

layout(location = 0) in vec2 position;

//output variable to the fragment shader
layout (location = 0) out vec3 outColour;

void main() {
    //const array of colors for the triangle
	const vec3 colours[3] = vec3[3](
		vec3(1.0f, 0.0f, 0.0f), //red
		vec3(0.0f, 1.0f, 0.0f), //green
		vec3(00.f, 0.0f, 1.0f)  //blue
	);

    gl_Position = vec4(position, 0.0, 1.0);
	outColour = colours[gl_VertexIndex];
}
