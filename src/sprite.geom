#version 150

layout(points) in;
layout(triangle_strip, max_vertices = 4) out;

out vec2 tex_coord;

uniform bool flip;

void main() {
    if(flip) {
        gl_Position = gl_in[0].gl_Position;
        tex_coord = vec2(16.0, 16.0);
        EmitVertex();
        gl_Position = gl_in[0].gl_Position + vec4(2.0/10.0, 0, 0, 0);
        tex_coord = vec2(0.0, 16.0);
        EmitVertex();
        gl_Position = gl_in[0].gl_Position + vec4(0, 2.0/9.0, 0, 0);
        tex_coord = vec2(16.0, 0);
        EmitVertex();
        gl_Position = gl_in[0].gl_Position + vec4(2.0/10.0, 2.0/9.0, 0, 0);
        tex_coord = vec2(0, 0);
        EmitVertex();
        EndPrimitive();
    } else {
        gl_Position = gl_in[0].gl_Position;
        tex_coord = vec2(0.0, 16.0);
        EmitVertex();
        gl_Position = gl_in[0].gl_Position + vec4(2.0/10.0, 0, 0, 0);
        tex_coord = vec2(16.0, 16.0);
        EmitVertex();
        gl_Position = gl_in[0].gl_Position + vec4(0, 2.0/9.0, 0, 0);
        tex_coord = vec2(0.0, 0.0);
        EmitVertex();
        gl_Position = gl_in[0].gl_Position + vec4(2.0/10.0, 2.0/9.0, 0, 0);
        tex_coord = vec2(16.0, 0.0);
        EmitVertex();
        EndPrimitive();
    }
}
