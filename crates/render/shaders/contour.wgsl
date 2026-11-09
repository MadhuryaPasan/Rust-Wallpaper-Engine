// Reusable contour function; audio controls line spacing.
fn contour(value: f32, width: f32) -> f32 {
    let line = abs(fract(value) - 0.5);
    return 1.0 - smoothstep(0.0, width, line);
}

fn shade(uv: vec2<f32>, time: f32, audio: f32, pointer: vec2<f32>) -> vec3<f32> {
    let p = (uv - 0.5) * vec2<f32>(1.6, 1.0);
    let field = length(p + (pointer - 0.5) * 0.1) * (16.0 + audio * 6.0) + sin(p.x * 5.0 + time * 0.2) * 1.8 + cos(p.y * 6.0 - time * 0.15);
    let line = contour(field, 0.06);
    let tint = mix(vec3<f32>(0.12, 0.42, 0.44), vec3<f32>(0.8, 0.38, 0.24), uv.x);
    return vec3<f32>(0.018, 0.025, 0.03) + line * tint * 0.6;
}
