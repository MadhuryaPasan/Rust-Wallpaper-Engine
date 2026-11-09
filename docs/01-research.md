# Research and product decisions

Research date: 9 November 2026. Working product name: **Serein** (subject to name clearance).

This is a design and engineering baseline, not a claim of completed feature parity. Official documentation establishes documented capabilities. Community posts identify failure modes and requests, not their prevalence. Public sources were read; no private community conversations were accessed. Recommendations below are our design decisions.

## What existing products establish

| Evidence | Implication for Serein |
| --- | --- |
| [Wallpaper Engine product](https://www.wallpaperengine.io/en): animated images, video, websites, sharing, extensive display support and automatic pausing. | These are baseline categories, not differentiators. |
| [Wallpaper Engine SceneScript](https://docs.wallpaperengine.io/en/scene/scenescript/reference.html): property scripts, cursor events, resize events and media integration. | Reusable behavior, event lifecycles and typed SDK documentation must be designed together. |
| [SceneScript audio](https://docs.wallpaperengine.io/en/scene/scenescript/reference/class/AudioBuffers.html) and [web audio](https://docs.wallpaperengine.io/en/web/audio/visualizer.html): bounded spectrum APIs and subscriber-driven delivery. | Subscription-driven audio and stable signal semantics belong in the runtime. |
| [Lively repository](https://github.com/lively-community/lively) and [system-data guide](https://github.com/lively-community/lively/wiki/Web-Guide-V-:-System-Data): multiple wallpaper players and system/audio/media integrations. | A single universal player is a poor fit; provide optional backends behind one project model. |
| [Lively performance guide](https://github.com/lively-community/lively/wiki/Performance): content drives cost, hardware video decoding matters, and fullscreen/battery rules reduce unnecessary work. | Benchmark complete workloads, not programming-language slogans. |
| [Wallpaper Engine multi-monitor release](https://store.steampowered.com/news/posts/?appgroupname=Wallpaper+Engine&appids=431960&enddate=1623502818&feed=steam_community_announcements): app rules, monitor-specific sound and bounded parallax. | Store display profiles, select one audible instance, define local versus desktop-wide coordinates. |

The creator scripting documentation, product pages, release notes and performance guides together establish the baseline. They do not establish that any product has every conceivable integration. Do not advertise Serein as uniquely offering a feature until a comparative test verifies that claim.

## Community signals

| Public report | Design response | Evidence limit |
| --- | --- | --- |
| [Lively #2464, Windows 24H2](https://github.com/lively-community/lively/discussions/2464) | Treat shell attachment as a compatibility adapter with a health check and recovery. | A discussion about an OS transition, not proof all versions fail. |
| [Lively #3314, failed parent attachment](https://github.com/lively-community/lively/issues/3314) | Restore a static desktop and show a diagnostic if attachment fails. Never leave a black, focus-stealing window. | Individual issue report. |
| [Lively #3322, desktop input](https://github.com/lively-community/lively/issues/3322) | Keep empty areas click-through; test paired input press/release and focus transitions. | Individual report; cause is not independently reproduced. |
| [Community playlist tool](https://www.reddit.com/r/projects/comments/1tu96z7/i_made_a_playlist_program_for_lively_wallpaper/) | Include local playlists, time/app rules and portable profiles in the baseline. | A creator's need, not a verified statement about current Lively support. |
| [Multi-monitor game slowdown report](https://www.reddit.com/r/wallpaperengine/comments/1j7ovg8) | Default to pausing all displays for games; offer explicit per-monitor overrides. | Hardware-specific anecdote, not a performance benchmark. |
| [Wallpaper Engine support discussion](https://www.reddit.com/r/wallpaperengine/comments/fayfui/wallpaper_engine_tech_support_questions_thread_faq/) | Add sleep/screensaver acceptance tests; never keep Windows awake just to animate. | Public support thread, not a measured failure rate. |

## Framework decision

Use **Iced 0.14 for the first native studio**, `wgpu` for native wallpapers, Rust domain/DSP libraries, and Windows APIs behind small adapters. Iced has message-based state updates, canvas, a text editor, theming and a custom GPU shader widget. The [official API](https://docs.rs/iced/0.14.0/iced/) and [release notes](https://github.com/iced-rs/iced/releases/tag/0.14.0) are the implementation references. State-driven views make ownership of editor state clearer than an ad hoc collection of immediate-mode panels. This is an architectural preference, not a measured performance win over egui.

| Option | Assessment |
| --- | --- |
| Iced | Selected for native Rust views and reusable layout/style functions. Rich IDE features, docking and assistive-technology behavior need dedicated work. |
| egui/eframe | Strong editor tooling and custom rendering. Viable, but achieving this product's layout consistency requires a disciplined component layer. Not rejected as inherently unsuitable. |
| Slint | Strong declarative UI candidate. [Maintainer blog](https://www.slint.dev/blog/making-slint-desktop-ready), [license explanation](https://github.com/slint-ui/slint/blob/master/LICENSE.md): desktop royalty-free use exists with attribution. Shader viewport and code editor integration would need an additional spike. Do not repeat the outdated claim that commercial use always requires a paid license. |
| Web-based studio | Excellent code-editor ecosystem, but a browser adds processes. Could be an optional later editor; it should never be required by a native wallpaper. |

Iced's screen-reader support must be validated before the consumer-release gate. Native widgets do not automatically imply accessible platform semantics. If the spike cannot meet keyboard and screen-reader requirements, reevaluate the studio toolkit before broadening the interface. Runtime crates remain unaffected.

## Windows feasibility and constraints

* [Microsoft SetParent](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setparent) documents window styles and DPI mismatches. Attaching under Explorer's WorkerW is an observed shell technique, **not a public wallpaper-host contract**. Isolate it, time out shell messages, test Explorer restart, and retain a static fallback. Never claim SetParent alone solves wallpaper hosting.
* [Microsoft loopback recording](https://learn.microsoft.com/en-us/windows/win32/coreaudio/loopback-recording) supports observing the render endpoint in shared mode. Handle silence, device removal and protected streams. Microphone input is separate and explicit.
* [Microsoft application-loopback sample](https://learn.microsoft.com/en-us/samples/microsoft/windows-classic-samples/applicationloopbackaudio-sample/) requires build 20348 or later. Per-app audio is capability-gated; ordinary system loopback remains the fallback.
* [WebView2 process model](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/process-model) includes browser, renderer and helper processes. Load it only for web content; measure its entire process group. Keep content isolation when considering environment sharing.
* [Wasmtime interruption](https://docs.wasmtime.dev/examples-interrupting-wasm.html) offers fuel and epoch mechanisms for CPU scripts. Neither limits a GPU shader. Runtime GPU work needs separate budgets and recovery; validation alone does not prevent a costly shader.
* EQ coefficients follow the [RBJ audio cookbook](https://webaudio.github.io/Audio-EQ-Cookbook/audio-eq-cookbook.html). A spectral display is not itself an equalizer; real PCM filtering must occur before the post-EQ analysis.

## What could make this product better

These are proposed strengths to validate, not exclusive inventions:

1. **Patch once, use everywhere:** a named audio bus can drive a shader uniform, mesh deformation, camera, particle emitter or timeline property through one signal contract.
2. **Performance as an editing tool:** show active passes, texture memory, CPU/GPU timing where supported, and the source of wakeups; offer a before/after comparison for an effect.
3. **Freeze beautifully:** a creator-defined resting composition replaces a paused mid-animation frame; deterministic snapshots and lightweight variants work offline.
4. **Display-aware composition:** safe areas, focal points, independent/span/clone modes and profiles for docked/undocked setups.
5. **Reproducible creation:** seeded simulations, recorded input fixtures, versioned SDK, reusable packages and publish-time validation.
6. **Honest complexity:** native scenes can be lean; arbitrary websites and heavy 3D cannot be promised the same budget. Profiles express a quality/resource tradeoff.

## Open research gates

Prototype Windows shell hosting across supported Windows builds; verify Iced with Narrator and high contrast; benchmark laptop iGPU/dGPU/hybrid paths; evaluate DXVA/Media Foundation interop; validate shader compilation isolation; investigate HDR/color management; test capture device changes. No numeric CPU/GPU superiority claim is justified before these gates.

