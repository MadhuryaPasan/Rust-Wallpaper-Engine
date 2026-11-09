// Aurora Drift: audio-reactive northern lights over stars and mountains.
// Paste into Serein > Advanced, then Compile (Ctrl+Enter).
// Tip: bus range 30-220 Hz, sensitivity 3x gives a nice bass-swell.

fn hash21(p: vec2<f32>) -> f32 {
    var q = fract(p * vec2<f32>(123.34, 456.21));
    q = q + vec2<f32>(dot(q, q + vec2<f32>(45.32, 45.32)));
    return fract(q.x * q.y);
}

fn vnoise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (vec2<f32>(3.0, 3.0) - 2.0 * f);
    let a = hash21(i);
    let b = hash21(i + vec2<f32>(1.0, 0.0));
    let c = hash21(i + vec2<f32>(0.0, 1.0));
    let d = hash21(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

fn fbm(p0: vec2<f32>) -> f32 {
    var p = p0;
    var sum = 0.0;
    var amp = 0.5;
    for (var i = 0; i < 4; i++) {
        sum += vnoise(p) * amp;
        p = p * 2.03 + vec2<f32>(17.1, 9.2);
        amp *= 0.5;
    }
    return sum;
}

fn star_field(p: vec2<f32>, time: f32) -> f32 {
    let g = p * 40.0;
    let cell = floor(g);
    let local = fract(g) - vec2<f32>(0.5, 0.5);
    let h = hash21(cell);
    let jitter = vec2<f32>(hash21(cell + vec2<f32>(7.7, 7.7)), hash21(cell + vec2<f32>(3.1, 3.1))) - vec2<f32>(0.5, 0.5);
    let d = length(local - jitter * 0.6);
    let twinkle = 0.6 + 0.4 * sin(time * (1.0 + h * 3.0) + h * 40.0);
    let present = step(0.93, h);
    return present * twinkle * exp(-d * 30.0);
}

fn shade(uv: vec2<f32>, time: f32, audio: f32, pointer: vec2<f32>) -> vec3<f32> {
    let aspect = signals.size.x / max(signals.size.y, 1.0);
    let p = (uv - vec2<f32>(0.5, 0.5)) * vec2<f32>(aspect, 1.0);
    let lvl = clamp(audio, 0.0, 1.0);
    let par = pointer - vec2<f32>(0.5, 0.5);

    // Night sky: deep blue at the top, slightly lighter near the horizon.
    let sky_top = vec3<f32>(0.002, 0.004, 0.018);
    let sky_low = vec3<f32>(0.010, 0.030, 0.060);
    var col = mix(sky_top, sky_low, smoothstep(0.0, 1.0, uv.y));

    // Stars (parallax: drift slightly against the pointer).
    let star = star_field(p + par * 0.03, time);
    col += vec3<f32>(0.9, 0.95, 1.0) * star * 0.8;

    // Three aurora curtains.
    var aurora = vec3<f32>(0.0, 0.0, 0.0);
    for (var k = 0; k < 3; k++) {
        let fk = f32(k);
        let x = p.x * (1.2 + fk * 0.35) + par.x * 0.12 * (fk + 1.0);
        let wave = fbm(vec2<f32>(x * 1.5 + fk * 5.0, time * 0.06 + fk * 3.0));
        let center = -0.16 + fk * 0.07 + (wave - 0.5) * 0.38 - lvl * 0.05 + par.y * 0.04;
        let dy = p.y - center;
        // Sharp lower edge, soft fade upward.
        let fall = select(2.6, 15.0, dy > 0.0);
        let curtain = exp(-abs(dy) * fall);
        // Vertical rays.
        let rays = 0.55 + 0.45 * vnoise(vec2<f32>(p.x * 55.0 + fk * 10.0, time * 0.25));
        let heat = clamp(-dy * 3.2 + fk * 0.25, 0.0, 1.0);
        let tint = mix(vec3<f32>(0.05, 0.85, 0.40), vec3<f32>(0.55, 0.15, 0.85), heat);
        let gain = 0.30 + lvl * 1.3;
        aurora += tint * curtain * rays * gain * (0.8 - fk * 0.15);
    }
    // Aurora outshines the stars behind it.
    col = col * (1.0 - clamp(length(aurora) * 0.6, 0.0, 0.8)) + aurora;

    // Soft glow that follows the pointer.
    let glow = exp(-length(uv - pointer) * 6.0);
    col += vec3<f32>(0.04, 0.12, 0.18) * glow * (0.5 + lvl);

    // Mountain silhouette with a faint rim light from the aurora.
    let ridge = 0.30 + fbm(vec2<f32>(p.x * 2.2 + 3.0, 1.7)) * 0.20 - 0.08;
    let inside = smoothstep(0.0, 0.004, p.y - ridge);
    let rim = exp(-abs(p.y - ridge) * 90.0);
    let ground = vec3<f32>(0.002, 0.004, 0.008);
    col = mix(col, ground, inside);
    col += vec3<f32>(0.03, 0.12, 0.10) * rim * (0.3 + lvl) * (1.0 - inside * 0.5);

    return col;
}
