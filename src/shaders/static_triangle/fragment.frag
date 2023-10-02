#version 460

layout(location = 0) in vec3 inColour;

layout(location = 0) out vec4 f_colour;

void main() {
    f_colour = vec4(inColour, 1.0);
}
