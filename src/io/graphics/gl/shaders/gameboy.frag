#version 430 core

in VS_OUTPUT {
   vec3 Color;
} IN;

out vec4 Color;

void main() { 
     Color = vec4(IN.Color, 1.0f);
}