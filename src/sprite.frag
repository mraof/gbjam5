#version 140

in vec2 tex_coord;
out vec4 color;

uniform sampler2D tex;
uniform sampler1D palette;

void main() {
	color = texelFetch(palette, int(texelFetch(tex, ivec2(tex_coord.x, tex_coord.y), 0).x * 256.0), 0);
}
