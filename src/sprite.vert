#version 140

in vec2 position;

out vec2 tex_coord;

uniform vec2 offset;
uniform sampler2D tex;
uniform bool flip;

void main() {
    ivec2 size = textureSize(tex, 0);
    tex_coord = vec2((int(flip) ^ int(position.x)) * size.x,  (int(position.y) ^ 1) * size.y);
	gl_Position = vec4(vec2(-1.0, -1.0) + (position * vec2(size) + offset) / vec2(80, 72), 0.0, 1.0);
}
