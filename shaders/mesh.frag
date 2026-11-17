#version 450
// Lit in model space (Z up): a key light from above and in front, a softer fill from the other side.
// With a section cut: the cut-away side is discarded, and the insides of the parts (back faces, seen
// through the cut) are drawn flat in the cut colour, so cut parts look solid.
layout(location = 0) in vec3 n;
layout(location = 1) in vec3 c;
layout(location = 2) in float side;
layout(location = 3) flat in float cutting;
layout(location = 0) out vec4 o;
void main() {
    if (side < 0.0) discard;
    if (cutting > 0.5 && !gl_FrontFacing) {
        o = vec4(pow(mix(c, vec3(0.85, 0.22, 0.15), 0.65), vec3(2.2)) * 0.8, 1.0);
        return;
    }
    vec3 nn = normalize(n);
    float key = abs(dot(nn, normalize(vec3(0.35, -0.45, 0.82)))); // two-sided: some faces may point inwards
    float fill = abs(dot(nn, normalize(vec3(-0.5, 0.6, 0.3))));
    vec3 albedo = pow(c, vec3(2.2)); // sRGB colour to linear (the swapchain is sRGB)
    o = vec4(albedo * (0.28 + 0.62 * key + 0.25 * fill), 1.0);
}
