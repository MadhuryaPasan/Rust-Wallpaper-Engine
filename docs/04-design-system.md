# Studio interaction and visual system

The original visual system below has been extended by the visual wallpaper library, Customize page, automatic output selector and real desktop playback controls. Current behavior is documented in the README and creator guide; the original analyzer-button and paused-preview descriptions below are historical design notes.

Serein uses a quiet, dark professional workspace: charcoal surfaces, warm white text, coral for the current primary action, and labelled cyan/amber/green traces for signal data. Color never carries a state by itself. The visual priority is the work being created, then its controls, then navigation.

## Tokens and components

Semantic tokens live in `crates/studio/src/theme.rs`. Background, surface, inset, border, text, secondary text, accent, on-accent, signal and warning each have their own role. Use a 4 px spacing scale, 8 px control radius and 12 px panel radius. No decorative shadow animation. Font roles: page title 28, section 18, body/control 14, caption 12; numerical readouts and code use a monospace face. Native text remains selectable where the widget supports it.

Shared button, navigation, panel, labelled control, badge and status treatments keep states consistent. Active navigation has an explicit marker and label. Primary buttons use dark text on coral. Use readable muted text rather than low-contrast gray. A measured contrast script verifies declared foreground/background pairs; rendered images and plots still need visual checks.

## Navigation and views

* **Library:** starter projects with real shader previews, opening/saving projects and the future local asset library. Importing a starter never silently replaces unsaved work.
* **Studio:** WGSL editor with a live GPU viewport, explicit Compile, diagnostics and last-good shader. Code changes do not execute on each keystroke.
* **Audio lab:** source state, real spectrum and EQ response, numbered bands, corresponding numeric controls, waveform and one reusable response-bus editor. The preview reacts to the same computed bus.
* **Displays:** detected physical monitor information, topology explanation and target layout modes. Until host assignment exists, display Apply is absent and configuration is labelled preview-only.
* **Performance:** policy simulator and future measurement contract. FPS ceiling is a setting, not a measured frame rate; no fabricated GPU load.
* **Roadmap/about:** implemented scope, next gates, SDK reference and known limitations.

## Adaptivity and accessibility

Desktop target is 960×700 logical pixels and above. Wide windows use a leading navigation rail and two content columns. Narrow/zoomed windows use abbreviated navigation and vertically stacked panels. The shell owns window minimums; all long content scrolls. Test at 100%, 150% and 200% Windows scaling and with long strings. This native desktop target does not claim a 320 CSS-pixel mobile layout.

Use standard widgets for editable values. Every canvas action has a labelled control alternative: EQ node drag also has band selection and frequency/gain/Q controls. Keyboard users can edit and reset without dragging. Focus styling, Tab/Shift-Tab, Enter/Space and Ctrl+S/Ctrl+Enter must be checked in the running app. No invisible primary action or icon-only navigation. Native screen-reader semantics require a real Narrator test and remain a release gate.

Preview motion starts only after an explicit Run preview action; source starts with generated test data clearly labelled and not playing through speakers. Pause is always visible. The production player must observe the Windows animation/reduced-motion preference and provide a static mode. Live analyzer and wallpaper motion have independent controls. No endlessly animated decorative chrome.

## States and writing

Use sentence case. Say Compile shader, Save project, Start system capture, Stop capture and Reset EQ. A saved message states the destination; a failure states what failed and the next action. Keep diagnostics visible until fixed/dismissed. Show source as Generated test signal, System output, Waiting for audio, or Capture failed. A moving test spectrum never masquerades as real music.

Unavailable capabilities have a short explanation rather than a working-looking button. Platform diagnostics must not expose private window titles, typed keys or audio recordings. Closing a development studio to its tray must make its lifetime clear; the later separate player is what enables the editor to exit from memory.

