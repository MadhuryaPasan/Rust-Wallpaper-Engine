# Product plan

Implementation update: the visual preset library, simple customization, automatic audio/device recovery, persistent image-parallax library, shared renderer and a separate Windows desktop player are now implemented. Close/minimize-to-tray preserves desktop playback. Read the [README](../README.md) and [current validation](08-current-validation.md) for the tested scope. The milestones below remain the broader product roadmap, not a claim that every acceptance condition is complete.

## Product and scope

Build a Windows-first wallpaper player and creator studio. Application logic and native rendering use Rust. WGSL is the creator shader language; optional web wallpapers necessarily use web technologies. Calling OS libraries does not make an application less Rust-based.

The full target is a multi-release product, not something to label complete after a UI prototype. This plan precedes implementation. The first delivery proves the native studio, reusable audio analysis, shader editing/preview and saved projects. The background host, media backends and full 3D editor have separate release gates.

## Requirements and acceptance matrix

| ID | Capability | Target gate | Acceptance condition |
| --- | --- | --- | --- |
| P01 | Tray, start at login, single instance, pause/resume, next wallpaper, quit | Player alpha | Close studio without stopping wallpaper; quit stops children and restores desktop. |
| P02 | Independent, clone, span and display groups | Player alpha | Portrait/ultrawide/mixed-DPI/negative origins; stable assignments after hotplug and docking. |
| P03 | Image + WGSL native wallpapers | Player alpha | Custom shader compiles off the render loop; errors preserve last good scene. |
| P04 | Hardware-decoded video, GIF/image sequences, playback speed and volume | Media beta | Document supported codecs, fallback path, loops and decoder recovery; no silent CPU decode. |
| P05 | Optional HTML/CSS/JS/WebGL/WebGPU web player and local web projects | Media beta | Native workloads do not start browser processes; permission-scoped bridge and persistent per-project storage. |
| P06 | 2D layers, masks, blend modes, depth/parallax, distortions, particles | Creator beta | Undoable parameter changes; reusable components; thumbnails and exposed user properties. |
| P07 | glTF 2.0 meshes, PBR materials, camera/light, animation and instancing | Creator beta | Models remain portable; bounds on triangles/textures; editor and runtime render the same result. |
| P08 | Audio routing and parametric EQ | Studio foundation → player alpha | Real pre/post filtering, logarithmic spectrum, waveform, bands, gain/Q/type, bypass and named buses. |
| P09 | Reusable modulation system | Creator beta | Sources → transforms → targets; one bus can drive many destinations without repeated capture/FFT. |
| P10 | Code and visual authoring | Studio foundation → creator beta | Shader diagnostics, last-good preview; later completion, graph/code parity, script debugger, reusable packages. |
| P11 | Mouse and opt-in key reactions | Creator beta | Monitor-local coordinates; desktop interaction preserves icons; no typed-text collection. |
| P12 | Playlists, schedules, app rules, portable presets and profiles | Media beta | Offline operation, deterministic priority, timezone/DST handling, explainable active rule. |
| P13 | Performance/quality policies | Player alpha | Fullscreen, covered monitor, battery, lock, remote session and display-off states tested. |
| P14 | Import/export and local library | Studio foundation → media beta | Versioned projects, validated relative paths, asset metadata; no proprietary-format promise. |
| P15 | Media metadata, clock/date/system metrics, optional weather/RGB | Later integrations | Each source permissioned, rate-limited, demand-driven and removable. |
| P16 | Screensaver, capture/export, recording | Later integrations | Obeys lock/session boundaries; recording runs only on explicit action. |
| P17 | Accessibility, localization, scaling and reduced motion | Every gate, release blocker | Keyboard alternatives, focus, labels, scalable panels; Narrator/contrast checks before distribution. |
| P18 | Recovery, updates, data migrations, diagnostics | Player alpha → release | Explorer/device loss recovery; crash-loop protection; rollback; redacted support bundle. |
| P19 | Community packages and discovery | After local product is reliable | Rights metadata, dependency locking, import checks, moderation/reporting and signed update channel. |

Feature categories are inspired by existing engines. This is not a compatibility promise for Steam Workshop, proprietary Wallpaper Engine formats, Shadertoy URLs or Lively packages. Each needs a separate feasibility/licensing/import assessment. Native executable wallpapers are excluded from automatic import; arbitrary code must not run on package installation.

## Creator experience

Three entry paths share one project format: customize a template, build with layers/nodes, or write code. The inspector exposes the same typed parameters for all three. No coding knowledge is required to route an existing Bass bus to particle size. Advanced creators can author reusable shaders and sandboxed logic.

Studio layout: project/assets/scene tree on the leading side; central preview with device/aspect preset; inspector on the trailing side; optional code, timeline, signal graph and diagnostics below. Save, undo and preview state stay in stable chrome. A command palette and searchable property inspector are planned. Panels have resettable layouts and keyboard alternatives to dragging.

## Audio lab

The reference image informs a logarithmic frequency grid, live spectrum and individually numbered EQ nodes. Add these functional distinctions:

* **Analysis EQ**, which changes what an effect hears; it never changes speaker output.
* Pre/post spectrum toggle and a separate time-domain waveform. They are different measurements.
* Seven initial bands spanning 20 Hz–20 kHz; peaking, low/high pass, low/high shelf and notch; adjustable center, gain and Q, per-band enabled state and global bypass. More bands can be added in a later document schema.
* Named response buses with frequency range, input gain, gate, attack/release, smoothing, normalization and response curve. Foundation starts with a configurable range, gain and attack/release; advanced gates remain explicit backlog.
* Raw RMS/peak, filtered spectrum, beat/onset confidence, stereo balance and optional phase-aware stereo mode. Beat/tempo must carry confidence; never report every transient as a reliable BPM.
* System mix initially; selected app and microphone later with clear source state. No recording or network transfer by default. Lost/disconnected sources show silence, not stale animation.
* Graph node dragging plus labelled numeric/slider controls and reset/undo. Spectrum displays can freeze independently from capture; hidden graph rendering stops.
* Presets are portable and linked to effect parameters. The project stores signal configuration rather than a captured copy of the user's music.

## Display and input model

Use stable display-device identifiers with a fallback matching strategy and a user-correctable assignment history. Keep physical desktop coordinates separate from logical studio pixels. Span uses the virtual desktop rectangle, including negative origins; per-display UVs stay 0–1. Preserve aspect ratio with crop/contain/cover/center/stretch and focal anchors. A monitor map previews portrait and asymmetric arrangements. Include refresh caps, resolution scale and audio ownership per group; HDR/SDR conversion is a distinct implementation gate.

Mouse position can be monitor-local, scene-local or desktop-wide. Global keyboard reactions require explicit opt-in, produce transient allowlisted key states only, and suspend on lock. Do not collect strings, maintain input history or synthesize desktop clicks as the default implementation. Editor input and desktop input use the same typed signal shape but different scopes.

## Performance contract

Proposed test targets, not measured promises: idle coordinator under 30 MiB private bytes; paused renderers submit no frames; simple 1080p native reference scene targets <1% total CPU on the designated reference machine, with CPU normalization recorded; background native scene process targets <100 MiB private bytes excluding driver allocations. These are feasibility gates, adjustable only with recorded benchmark evidence.

Measure warm and cold start, time to desktop, private bytes/working set, GPU allocation, CPU time, GPU timing where supported, frame pacing, process count, decoder engine, wakeups and incremental wall/battery energy. Compare the **same visual workload**, resolution, FPS, machine, driver, display wiring and editor-closed state. Report p50/p95 and long-session drift; Task Manager GPU percentage alone is inadequate. Web and complex 3D have separate budgets.

Quality profiles: Eco (15 FPS, reduced scale/effects), Balanced (30 FPS), Smooth (up to 60 FPS), Custom (explicit ceiling). Static or unchanged content does not redraw. A budget breach first reduces optional effects/scale, then freezes to a still with an explanation. Foreground games default to pause all displays; experts can override by monitor. Pausing retains memory, unloading releases runtime resources. These are distinct actions.

## Delivery sequence

1. **Foundation — current implementation:** native studio visual system, testable domain/DSP, seven-band audio lab, explicit test input/system capture, shader editor and GPU preview, validated project persistence, monitor inspection, policy simulator and tray prototype. Every unavailable production feature is labelled.
2. **Player alpha:** separate coordinator/renderer binaries, validated local IPC, desktop attachment and static fallback, native wallpaper application, tray ownership, startup, display recovery, subscription-driven capture and telemetry. Prove editor exits from memory.
3. **Media beta:** video, web plugin, import wizard, library, playlists, rules and display profiles.
4. **Creator beta:** actual scene graph, glTF/PBR, nodes, timeline, reusable components, Wasm logic, versioned SDK and package tooling.
5. **Release qualification:** benchmark comparison, soak tests, hardware matrix, Narrator/keyboard/high contrast, localization, signed installer/update rollback, security review and support documentation.

The first delivery must not present simulated CPU/GPU figures, pretend display assignments, decorative EQ knobs, a fake working Apply button or an implemented 3D editor. Designs for later milestones belong in documentation or labelled roadmap surfaces.

