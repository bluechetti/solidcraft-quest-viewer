#version 450
#extension GL_EXT_multiview : enable
// tint: rgb = highlight colour, a = how much of it replaces the vertex colour (0 = none).
// plane: the section cut in model units (keep where dot(xyz, pos) + w >= 0); (0, 0, 0, 1) = no cut.
layout(push_constant) uniform PC { mat4 mvp[2]; vec4 tint; vec4 plane; } pc;
layout(location = 0) in vec3 pos;
layout(location = 1) in vec3 normal;
layout(location = 2) in vec4 colour;
layout(location = 0) out vec3 n;
layout(location = 1) out vec3 c;
layout(location = 2) out float side;
layout(location = 3) flat out float cutting;
void main() {
    n = normal;
    c = mix(colour.rgb, pc.tint.rgb, pc.tint.a);
    side = dot(pc.plane.xyz, pos) + pc.plane.w;
    cutting = dot(pc.plane.xyz, pc.plane.xyz) > 0.0 ? 1.0 : 0.0;
    gl_Position = pc.mvp[gl_ViewIndex] * vec4(pos, 1.0);
}
