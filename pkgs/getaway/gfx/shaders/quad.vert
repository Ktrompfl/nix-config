#version 450

layout(push_constant) uniform Push {
    vec4 dst;
    vec4 uv;
    vec4 color;
    uint fill;
    uint transform;
    uint uv_transform;
} push;

layout(location = 0) out vec2 uv;

void main() {
    vec2 c = vec2(gl_VertexIndex & 1, gl_VertexIndex >> 1);
    vec2 sampled[8] = vec2[](c, vec2(c.y, 1.0 - c.x), 1.0 - c, vec2(1.0 - c.y, c.x), vec2(1.0 - c.x, c.y), c.yx,
                             vec2(c.x, 1.0 - c.y), 1.0 - c.yx);
    uv = push.uv.xy + sampled[push.uv_transform & 7u] * push.uv.zw;
    vec2 p = push.dst.xy + c * push.dst.zw;
    vec2 rotated[4] = vec2[](p, vec2(p.y, -p.x), -p, vec2(-p.y, p.x));
    gl_Position = vec4(rotated[push.transform & 3u], 0.0, 1.0);
}
