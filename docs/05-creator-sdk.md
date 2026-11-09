# Creator SDK direction

This document records the original prototype API and future SDK design. For the **current implemented API**, image bindings, controls and complete compiling examples, use [the creator guide](07-creator-guide.md). The current shared signal uniform is 80 bytes; the 32-byte interface described below is historical.

The foundation implements the **fragment preview API below**. Scene nodes, Wasm, named package imports, compute passes and 3D object APIs in the remaining sections are design proposals, not callable features yet.

## Implemented preview API v0

```wgsl
fn soft_pulse(level: f32) -> f32 {
    return smoothstep(0.05, 0.85, level);
}

fn shade(
    uv: vec2<f32>,
    time: f32,
    audio: f32,
    pointer: vec2<f32>
) -> vec3<f32> {
    let glow = soft_pulse(audio);
    let distance = length(uv - pointer);
    return vec3<f32>(0.06, 0.3, 0.45) * exp(-distance * 5.0) * (0.3 + glow);
}
```

`uv` is normalized viewport position with top-left origin. `time` advances in seconds only while preview motion runs. `audio` is the configured post-EQ response bus in 0–1. `pointer` is viewport-local UV when enabled and inside the preview; otherwise it is centered. The host owns the vertex stage and one 32-byte signal uniform. The author provides a linear RGB color; the preview target handles its output encoding. No additional bindings or entry points are accepted in v0. The preview currently adapts arbitrary viewport sizes by UV, not a fixed aspect-ratio promise.

Compile is explicit. Naga parse/type validation happens before the source replaces the preview's last-good source. The GPU pipeline is created on the next draw. This development preview is **not a secure shader sandbox**, and compilation is not yet isolated from the rendering process. Do not distribute it as an untrusted package runner.

Project JSON stores authoring source, not compiled GPU caches. Save can preserve unfinished shader work; opening currently requires a compiling shader. Export a working project before sharing, or send the WGSL source to a contributor for repair. A future recovery importer should open invalid source in edit-only mode without executing it.

## Proposed SDK v1

The runtime snapshot should offer reusable functions without granting arbitrary OS access:

| Source | Shape | Subscription |
| --- | --- | --- |
| Clock | elapsed time, bounded delta, local time and day/night signal | animation/time |
| Audio | RMS, peak, raw/post spectrum, named normalized buses, beat/onset confidence | explicit audio source grant |
| Pointer | local UV, desktop coordinates, velocity, inside/valid state | mouse position |
| Keys | allowlisted transient key states, no strings/history | explicit key-reactivity grant |
| Media | optional title/artist/artwork/playback state | media permission |
| System | bounded CPU/load/battery samples with units and timestamps | low-rate metrics permission |
| Scene | target parameters, camera transforms, materials, particle controls | local scene ownership |

All sources have valid/stale flags and monotonic timestamps. A disconnected audio device decays to zero and sets stale; it does not hold the last beat forever. A pointer outside the desktop is invalid rather than a surprising corner coordinate. Lock clears transient key state.

### A reusable modulation patch

Proposed patch: `Audio bus Bass pulse -> noise gate -> soft curve -> remap 0.2…1.4 -> sphere emission`. The same bus also drives `particle spawn rate` and `camera shake`, with independent transforms and limits. Graph/code authors see the same parameter IDs, units and defaults.

Core transforms: clamp, remap, smoothstep, attack/release envelope, spring, dead zone, hysteresis, pulse, hold, seeded noise, LFO and curve lookup. Expensive transforms declare cost and evaluation rate. Event sources do not masquerade as continuously sampled floats. Graph cycles require explicit bounded delay/state nodes.

Package functions declare input/output types, SDK compatibility, permissions and resource hints. Imports resolve from pinned local dependencies. Documentation/examples are shipped with the SDK, so completion and external editors use the exact same contract. Unused providers and transforms are pruned.

### Code and graph workflows

Keep native GPU work in WGSL. Use visual nodes for common bindings and Wasm for optional advanced CPU logic. A Rust author can compile a Wasm component with a published SDK; casual creators never need a compiler. Do not execute imported native DLLs or compile Rust inside the background player.

The studio should provide diagnostics at source locations, completion, hover help, snippets, find/replace, formatting, undo/redo, split views, a resource inspector, parameter watch and deterministic input replay. Timeline and node changes join the same undo history. Failed compilation preserves output and shows the exact stale revision.

### Audio beyond foundation

Seven bands are the first editable set, not a product-wide limit. Later support includes adding/removing bands, pass-filter slopes, independent named buses, per-app/mic sources, channel selection/stereo analysis, gating, RMS/peak modes, auto-level, curves, onset/tempo confidence and spectrum/waveform freeze.

Foundation uses a 2048-sample FFT at 48 kHz: about 23.4 Hz bin spacing and a 42.7 ms window. A logarithmic graph does not create more bass resolution. Planned quality choices use larger windows or multiresolution analysis when subscribed, and describe the resulting latency. Let users choose a faster transient response versus finer low-frequency analysis. Never modify system playback EQ to achieve analysis filtering.

The spectrum shows input or filtered magnitude in dBFS. The EQ curve separately shows filter gain in dB. The waveform is raw time-domain input. Label axes/legends so these distinct units are not confused. Provide a true spectrum dBFS axis and keyboard-addressable numeric controls as the graph matures.
