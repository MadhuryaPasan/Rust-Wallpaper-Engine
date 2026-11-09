// Each silhouette has its own depth factor instead of moving the whole image.
fn shade(uv: vec2<f32>, time: f32, audio: f32, pointer: vec2<f32>) -> vec3<f32> {
    let p = uv + (pointer - 0.5) * 0.008;
    var color = mix(vec3<f32>(0.025, 0.035, 0.075), vec3<f32>(0.5, 0.23, 0.17), pow(p.y, 0.7));
    let sun = exp(-length((p - vec2<f32>(0.72, 0.32)) * vec2<f32>(1.65, 1.0)) * 24.0);
    color += sun * vec3<f32>(0.9, 0.38, 0.15) * (1.0 + audio * 0.8);
    for (var i = 0; i < 6; i++) {
        let depth = f32(i) / 5.0;
        let x = p.x + (pointer.x - 0.5) * 0.035 * (0.5 + depth * 2.0);
        let ridge = 0.48 + depth * 0.085 + sin(x * (4.0 + depth * 2.0) + f32(i) * 1.8) * (0.035 + depth * 0.04) + sin(x * 11.0 + depth * 8.0) * 0.018;
        let layer = mix(vec3<f32>(0.35, 0.18, 0.17), vec3<f32>(0.025, 0.035, 0.06), depth);
        color = mix(color, layer, smoothstep(ridge - 0.002, ridge + 0.002, p.y));
    }
    return color;
}
