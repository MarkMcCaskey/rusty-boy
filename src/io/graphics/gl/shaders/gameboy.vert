#version 430 core

layout(location = 0) in vec2 vert_xy;
layout(location = 1) in vec3 Color;

out VS_OUTPUT {
    vec3 Color;
} OUT;

void main() { 
     gl_Position = vec4(vert_xy, 0.0, 1.0);
     OUT.Color = Color;
}
