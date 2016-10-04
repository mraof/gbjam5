#version 150

in vec2 position;

uniform vec2 offset;

void main() {
	gl_Position = vec4(vec2(-1.0, -1.0) + (position + offset) / vec2(80, 72), 0.0, 1.0);
}
