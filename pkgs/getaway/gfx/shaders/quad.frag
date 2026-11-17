#version 450

layout(push_constant) uniform Push {
    vec4 dst;
    vec4 uv;
    vec4 color;
    uint fill;
    uint transform;
    uint uv_transform;
} push;

layout(set = 0, binding = 0) uniform sampler2D tex;
layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 color;

void main() {
    color = push.fill == 1u ? push.color : texture(tex, uv) * push.color.a;
}
