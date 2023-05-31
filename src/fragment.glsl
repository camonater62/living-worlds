#version 330 core
    
uniform sampler1D palette;
uniform sampler2D color_indices;

in vec2 uv;

out vec4 fragColor;

void main() {    
    vec2 flippedUV = vec2(uv.x, 1.0 - uv.y);
    float index = texture(color_indices, flippedUV).r;
    vec3 color = texture(palette, index).rgb;

    fragColor = vec4(color, 1.0);
}