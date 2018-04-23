#version 430 core

layout(location = 0) in vec2 vert_xy;
out vec4 frag_rgba;

void main() { 
	frag_rgba = vec(1,1,0,1);
}
