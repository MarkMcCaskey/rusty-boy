#version 430 core

layout(location = 0) in vec2 vert_xy;
layout(location = 1) in vec3 Color;
layout(location = 2) in vec2 Texcoord;

out VS_OUTPUT {
    vec3 Color;
    vec2 Texcoord;
} OUT;

void main() { 
     gl_Position = vec4(vert_xy, 0.0, 1.0);
     OUT.Color = Color;
     OUT.Texcoord = Texcoord;
}
