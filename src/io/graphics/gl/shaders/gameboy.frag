#version 430 core

in VS_OUTPUT {
   vec3 Color;
   vec2 Texcoord;
} IN;

out vec4 Color;

uniform sampler2D tex;

void main() { 
     Color = texture(tex, IN.Texcoord); //vec4(IN.Color, 1.0f);
}