# Creating wallpapers and effects in Serein

You can use every included wallpaper without writing code. Code is optional and lives in **Advanced**.

## Start with the visual controls

1. Open **Library** and select a preview tile.
2. Open **Customize**. Adjust speed, brightness, scene zoom, audio reaction, mouse depth and color treatment. Controls are saved locally for that preset.
3. Choose the display mode and select **Apply wallpaper**. The desktop player runs separately from the studio.
4. Close or minimize Serein to keep the wallpaper running from the tray. Use the tray menu to reopen, pause/resume, stop, or exit.

Applying does not replace your Windows background setting. Serein draws behind the desktop icons; stopping it reveals the original background again. Primary, selected-display, duplicate and span modes are available. A display-mode change takes effect when you select Apply again.

## Audio response without setup buttons

Open **Audio lab** and choose an audio output. **Default output** follows the Windows default playback device. Selecting a named output pins analysis to that endpoint, even if the Windows default changes. If it disconnects, Serein waits and reconnects automatically. It never silently substitutes another device for a pinned selection.

The EQ filters a copy of the output audio **for analysis**. It does not change what your speakers play. A microphone is not opened. No audio recording is created.

The white curve is filter gain, measured against the left dB scale. The cyan spectrum uses the right dBFS scale. The waveform is the time-domain input. Pre/post switches let you inspect the effect of filtering.

To react mainly to kick drums or bass, start with a bus range of **30–220 Hz**, sensitivity **3×**, attack **25 ms** and release **240 ms**. For vocals, try **250–2500 Hz**. For percussion/air, try **4000–16000 Hz**. These are starting presets, not a universal separation of instruments. Overlapping instruments cannot be isolated perfectly by frequency alone.

`audio` in a shader is the normalized output of the current response bus. The same filtered signal drives the preview and the applied wallpaper. While the studio is hidden, its analyzer stops; the active desktop player keeps the analysis it needs. When playback pauses automatically for fullscreen/battery/lock, capture pauses with it and resumes automatically.

## Your first shader

Choose a preset, open **Advanced**, replace the editor contents with this example, and select **Compile shader** (`Ctrl+Enter`).

```wgsl
fn shade(
    uv: vec2<f32>,
    time: f32,
    audio: f32,
    pointer: vec2<f32>
) -> vec3<f32> {
    let distance = length(uv - pointer);
    let glow = exp(-distance * 7.0);
    let pulse = 0.35 + audio * 1.5;
    let color = vec3<f32>(0.08, 0.45, 0.7);
    return color * glow * pulse;
}
```

This creates a mouse-following glow that brightens with your selected audio range. **Apply wallpaper** installs the current valid shader on the desktop. If you compile a new valid shader while that same wallpaper is applied, it updates automatically. Selecting a different library tile only changes the studio selection until you apply it.

Invalid code leaves the last good preview and wallpaper running. Undo/redo are available in the editor (`Ctrl+Z`, `Ctrl+Y` or `Ctrl+Shift+Z`). The editor is an initial WGSL workspace, not yet a full IDE with completion/debugging.

## Shader inputs and settings

`shade()` returns **linear RGB** values. Serein converts them to sRGB exactly once
for display, whether the target is the live canvas, a catalog image or the desktop.
Do not add a gamma correction to your final shader return value. Imported color
images are decoded from sRGB automatically; depth maps stay linear. The host
reserves names beginning with `serein_` for its output conversion.

| Input | Meaning |
| --- | --- |
| `uv` | View coordinates with top-left origin. Normally 0–1, transformed by scene zoom; values can extend beyond that range when zooming out. In span mode the composition continues across displays. |
| `time` | Animation time in seconds, multiplied by your speed setting. It stops while wallpaper playback is paused. |
| `audio` | Response-bus output, scaled by the audio-reaction setting. It can reach 5 with that setting at its maximum, so clamp it if your effect requires 0–1. |
| `pointer` | Pointer position centered around 0.5, scaled by mouse-depth strength. At default strength it follows normalized coordinates. Stronger depth can extend it beyond 0–1. Set depth to zero to center it. |

Return a **linear RGB** `vec3<f32>`. The host applies brightness/color treatment and output encoding. Use `signals.size.x / signals.size.y` if geometry needs the desktop aspect ratio. Raw normalized pointer UV is available as `signals.pointer` when you need it before depth scaling. `signals.detail` is the scene-zoom setting; the host has already applied it to `uv`.

The host supplies the vertex stage and image/depth bindings. Do not add `@vertex`, `@fragment`, `@compute`, or `@group/@binding` resources to the editor source. Native executable plugins and unrestricted filesystem/network calls are not part of this shader API.

## Reusable functions

Put functions above `shade()` and call them from any effect. For example:

```wgsl
fn soft_gate(level: f32, threshold: f32) -> f32 {
    return smoothstep(threshold, threshold + 0.15, level);
}

fn remap01(value: f32, low: f32, high: f32) -> f32 {
    return low + clamp(value, 0.0, 1.0) * (high - low);
}

fn rotate2(p: vec2<f32>, angle: f32) -> vec2<f32> {
    let c = cos(angle);
    let s = sin(angle);
    return vec2<f32>(p.x * c - p.y * s, p.x * s + p.y * c);
}

fn shade(uv: vec2<f32>, time: f32, audio: f32, pointer: vec2<f32>) -> vec3<f32> {
    let position = rotate2(uv - 0.5, time * 0.15);
    let response = soft_gate(audio, 0.05);
    let radius = remap01(response, 0.15, 0.35);
    let ring = exp(-abs(length(position) - radius) * 90.0);
    return vec3<f32>(0.25, 0.65, 0.9) * ring;
}
```

The audio service already applies attack/release, so you do not need to recreate audio smoothing in each shader. Shader-local variables do not persist between frames. Persistent CPU scripts, named package imports, visual modulation nodes and arbitrary user-defined inspector schemas remain future SDK work; ordinary reusable WGSL functions work today.

## A small 3D effect

This is actual sphere geometry evaluated in a fragment shader, rather than an imported mesh:

```wgsl
fn shade(uv: vec2<f32>, time: f32, audio: f32, pointer: vec2<f32>) -> vec3<f32> {
    let aspect = signals.size.x / max(signals.size.y, 1.0);
    let p = (uv - 0.5) * vec2<f32>(aspect, 1.0);
    let center = (pointer - 0.5) * 0.06;
    let q = p - center;
    let radius = 0.23 + clamp(audio, 0.0, 1.0) * 0.04;
    let depth_squared = radius * radius - dot(q, q);
    if depth_squared <= 0.0 {
        return vec3<f32>(0.015, 0.025, 0.04);
    }
    let normal = normalize(vec3<f32>(q, sqrt(depth_squared)));
    let light_direction = normalize(vec3<f32>(-0.5, -0.7, 1.0));
    let diffuse = max(dot(normal, light_direction), 0.0);
    let surface = vec3<f32>(0.15, 0.45, 0.7);
    return surface * (0.12 + diffuse * 0.88);
}
```

For distance-field 3D, open **Liquid chrome** or **Prism bloom** in Advanced. Their source shows bounded ray marching, distance functions, surface normals and shading. Keep march counts bounded and test at the intended resolution. A full-screen shader is evaluated per pixel: one extra expensive loop can cost more than the entire app shell.

## Convert an image into mouse depth

1. In Library select **Import an image**. PNG, JPEG and WebP are supported, up to 8192 pixels per side within the decoder allocation limit.
2. Open Customize and set **Mouse depth** to a subtle value, starting around **0.02–0.04**.
3. Apply the wallpaper and move your mouse over the desktop.
4. If you have a depth map, choose **Choose a depth map**. White is near and black is far; it should align with the source image.

Without a depth map, Serein uses a blurred compositional/luminance approximation generated locally. This can make a photograph feel layered, but it does not recognize objects, recover hidden surfaces or create a true 3D mesh. High strength can stretch edges. A hand-painted or externally generated depth map generally gives better results. No machine-learning model is downloaded or claimed.

Inside a custom image shader, `image_parallax(uv, pointer)` returns the warped image color. You can add an audio-controlled glow or tint around that result. Imported assets are copied to Serein's local data folder so moving the original file does not break the wallpaper.

## Files, limits and performance

Settings and per-preset customizations live under `%LOCALAPPDATA%\Serein`. Export project writes JSON containing source/settings and local image references; it is **not** a portable asset bundle yet. Copy referenced images and update their paths when sharing a photo project with another machine. Source-only shader projects are self-contained.

The current FFT uses a 2048-sample window at 48 kHz, approximately 23.4 Hz bin spacing and 42.7 ms of audio. Logarithmic plotting does not create additional low-frequency resolution.

Try 30 FPS first, then 15 FPS for lower animation cost. Use the smallest number of shader iterations that achieves the look. Prefer baked/static content when it conveys the same result, and leave fullscreen/battery pause enabled unless you need continuous playback.

WGSL validation catches source/type/interface errors. It does not prove that a shader is inexpensive or safely bounded on the GPU. Run your own shaders locally; secure untrusted-package execution is not provided by this development build.
