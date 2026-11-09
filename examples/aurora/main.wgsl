// Serein fragment SDK v0: time, audio and pointer arrive from the same signal frame.
// Edit shade(); the studio supplies the vertex stage and bind group.
fn shade(uv: vec2<f32>, time: f32, audio: f32, pointer: vec2<f32>) -> vec3<f32> {
    let p = (uv - 0.5) * vec2<f32>(1.6, 1.0);
    let drift = time * 0.15;
    let bend = 0.18 * sin(p.x * 4.0 + drift) + 0.09 * cos(p.x * 7.0 - drift);
    let ridge = p.y + bend + (pointer.y - 0.5) * 0.08;
    let ribbon = exp(-abs(ridge) * (13.0 - audio * 4.0));
    let halo = exp(-abs(ridge) * 3.8);
    let color = mix(vec3<f32>(0.12, 0.48, 0.63), vec3<f32>(0.92, 0.29, 0.19), smoothstep(-0.6, 0.6, p.x));
    let grain = fract(sin(dot(uv, vec2<f32>(127.1, 311.7))) * 43758.5453) * 0.012;
    let background = vec3<f32>(0.018, 0.028, 0.042) + grain;
    return background + color * (ribbon * (0.55 + audio * 0.9) + halo * 0.16);
}
