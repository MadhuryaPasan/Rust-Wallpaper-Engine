# Validation record

Historical record for the first prototype. The current feature set and follow-up checks are in [current validation](08-current-validation.md).

Date: 9 November 2026. Tested on the available Windows desktop with Rust 1.99.1, Iced 0.14 and wgpu 27. This is a development-foundation review, not production qualification.

## Automated checks

* `cargo test --workspace`: 18 tests passed; the one hardware-dependent loopback test is ignored by default. Tests cover EQ response, identity/bypass behavior, real post-filter spectral changes, audio range isolation, envelope decay, worker stop/cleanup, malformed/future project rejection, project persistence, topology bounds, playback priority, all bundled WGSL and shader ABI errors.
* `cargo test -p serein-audio -- --ignored`: the Windows loopback initialization/release test passed separately on the available output endpoint. This is not a device-switch, protected-content or long-session test.
* `cargo clippy --workspace --all-targets -- -D warnings`: passed after fixes.
* `cargo fmt --all --check`: passed after formatting.
* `cargo build -p serein-studio`: native executable built. An earlier rebuild failed because the running executable was locked; closing the development app resolved that, and the rebuild passed.
* `python scripts/check_contrast.py`: declared text pairs pass 4.5:1. Measured text/surface 14.50:1, secondary/surface 7.82:1, secondary/hover 6.37:1, accent/surface 8.65:1, primary button text/accent 9.71:1, signal/surface 10.21:1 and warning/surface 11.06:1. This is a token check, not a whole-screen image/graphics audit.

## Native inspection

Launched the compiled Windows app and inspected actual GPU output, rather than an HTML mockup. The shipped shader renders. Starting generated test analysis produces a spectrum; Run preview produces time/audio-responsive motion. Numbered band selection updates the inspector. Ctrl+K opens and focuses the command input; entering `analyzer` starts analysis. Studio displays editable highlighted WGSL. The compact `--compact` window stacks content and shows a working vertical scroll path with no horizontal clipping in the inspected view.

After correcting the native input targeting and broadening plot selection, a graph drag visibly changed a band frequency/gain and its filter response. A separate test verifies window-to-plot coordinates, stale-cursor handling and release cleanup. Exact native pointer fidelity across scaling factors is not verified. Source parse/type rejection is tested, but arbitrary GPU-load recovery is not.

The wide initial window extended beyond the available capture area. Startup was changed to a smaller centered window; the compact layout was then inspected. Tests at 150%/200% Windows scaling and long translated labels remain unverified. A native 1000-pixel-wide window is not a claim of mobile-web 320 CSS-pixel support.

## Interface review coverage

Project conventions inspected: `CONTRIBUTING.md`, `docs/03-architecture.md`, `docs/04-design-system.md`; native Iced components/styles in `views.rs`, `plots.rs` and `theme.rs`.

| Domain | Evidence inspected | Result |
| --- | --- | --- |
| Accessibility | Native window UI Automation tree, command input, standard text controls, paused initial preview | HIGH: content is absent from the platform accessibility tree. Full keyboard focus/activation and high contrast remain unverified. |
| Layout | Running Audio lab and Studio; compact 1000×740 launch; responsive branches in `views.rs` | Panel stacking inspected. 150%/200% scaling and complete translated states not verified. |
| Writing | Source/status labels, scope notices, diagnostics, dirty-work guard in code | Implemented/planned and test/live states distinguished. File-dialog failure/cancel flows not all exercised through UI. |
| Typography | Native Segoe UI, monospace metrics/editor, 12/14/18/30 role scale | Inspected default rendering; alternate fonts/locales and increased text scale not verified. |
| Colors | Actual dark UI and computed semantic text pairs | Text pairs pass. Native high contrast and every graph/control boundary not verified. |
| UI polish | Running controls, active navigation, panel/plot conventions | Consistent native theme inspected. Empty Undo/Redo histories now disable their actions. Full focus behavior still needs validation. |

| Severity | Domain | Location | Before | After / required action | Why |
| --- | --- | --- | --- | --- | --- |
| HIGH | Accessibility | Native studio content; `crates/studio/src/main.rs` toolkit integration | UI Automation exposes the title bar/system buttons, not creator controls | Prove a supported accessibility bridge or reevaluate the production toolkit before broadening it | A screen-reader user cannot access the creator workflow. A command palette does not solve this. |

**Verdict: Block consumer release.** The design/foundation is reviewable and runnable. Accessibility, the unverified checks above and the production player gates must be resolved before claiming a complete application.

## Deliberately unimplemented

Desktop wallpaper attachment/application; separate coordinator/renderer IPC; actual OS fullscreen/battery monitoring; hardware video/web playback; scene/mesh/node/timeline authoring; Wasm execution; global input reactions; community gallery/package archives; installation/startup/update signing; HDR; production multi-monitor assignment and recovery. Display data is real; playback decisions on the Performance page are a simulation.

The development tray keeps the studio in memory and stops its analysis/preview on hide. It is not evidence of the future lightweight player. No CPU/GPU/RAM/energy superiority benchmark was run or claimed.
