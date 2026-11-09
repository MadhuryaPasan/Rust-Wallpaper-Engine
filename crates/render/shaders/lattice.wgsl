fn shade(uv: vec2<f32>, time: f32, audio: f32, pointer: vec2<f32>) -> vec3<f32> {
    let p = uv - 0.5; let horizon = 0.52;
    var color = mix(vec3<f32>(0.035, 0.01, 0.09), vec3<f32>(0.15, 0.02, 0.13), uv.y);
    let sun_pos = vec2<f32>(0.5, 0.37); let sun_distance = length((uv - sun_pos) * vec2<f32>(1.6, 1.0));
    let sun = 1.0 - smoothstep(0.145, 0.15, sun_distance);
    let cut = step(0.18, fract(uv.y * 55.0));
    color += sun * cut * mix(vec3<f32>(1.0, 0.18, 0.04), vec3<f32>(0.6, 0.02, 0.45), smoothstep(0.2, 0.53, uv.y));
    if uv.y > horizon {
        let z = 1.0 / max(uv.y - horizon, 0.008);
        let x = p.x * z * 2.0 + (pointer.x - 0.5) * 0.2;
        let line_x = pow(1.0 - abs(fract(x) - 0.5) * 2.0, 36.0);
        let line_z = pow(1.0 - abs(fract(z * 0.6 + time * 0.12) - 0.5) * 2.0, 24.0);
        color = vec3<f32>(0.012, 0.012, 0.04) + max(line_x, line_z) * vec3<f32>(0.08, 0.45, 0.65) * (0.5 + audio * 1.2) * smoothstep(horizon, 0.7, uv.y);
    }
    return color;
}
