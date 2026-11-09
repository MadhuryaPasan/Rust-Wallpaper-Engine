fn crystal_distance(p: vec3<f32>, time: f32) -> f32 {
    let angle = time * 0.18; let c = cos(angle); let s = sin(angle);
    let q = abs(vec3<f32>(p.x * c - p.z * s, p.y, p.x * s + p.z * c));
    return (q.x + q.y * 0.65 + q.z - 0.7) * 0.577;
}
fn shade(uv: vec2<f32>, time: f32, audio: f32, pointer: vec2<f32>) -> vec3<f32> {
    let aspect = signals.size.x / max(signals.size.y, 1.0);
    let ray = normalize(vec3<f32>((uv - 0.5) * vec2<f32>(aspect, 1.0) * 2.2, -2.7));
    let origin = vec3<f32>((pointer.x - 0.5) * 0.18, (pointer.y - 0.5) * 0.18, 3.0);
    var travel = 0.0; var hit = false;
    for (var i = 0; i < 40; i++) { let distance = crystal_distance(origin + ray * travel, time); if distance < 0.001 { hit = true; break; } travel += distance; if travel > 6.0 { break; } }
    var color = vec3<f32>(0.016, 0.018, 0.04);
    if hit {
        let p = origin + ray * travel; let e = 0.003;
        let n = normalize(vec3<f32>(crystal_distance(p + vec3<f32>(e, 0.0, 0.0), time) - crystal_distance(p - vec3<f32>(e, 0.0, 0.0), time), crystal_distance(p + vec3<f32>(0.0, e, 0.0), time) - crystal_distance(p - vec3<f32>(0.0, e, 0.0), time), crystal_distance(p + vec3<f32>(0.0, 0.0, e), time) - crystal_distance(p - vec3<f32>(0.0, 0.0, e), time)));
        let light = max(dot(n, normalize(vec3<f32>(-0.4, -0.8, 1.0))), 0.0);
        color = (n * 0.5 + 0.5) * vec3<f32>(0.45, 0.35, 0.8) * (0.25 + light);
        color += pow(1.0 - abs(dot(n, -ray)), 2.0) * vec3<f32>(0.1, 0.5, 0.7) * (0.5 + audio);
    }
    return color;
}
