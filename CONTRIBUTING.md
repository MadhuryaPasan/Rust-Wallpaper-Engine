# Contributing to Serein

Start with `docs/02-product-plan.md` and `docs/03-architecture.md`. The current implementation is a foundation; work toward a complete vertical slice rather than adding working-looking controls for unimplemented capabilities.

## Ownership

Domain validation, geometry and playback policy belong in `serein-domain`. DSP/capture belongs in `serein-audio`. Studio interaction, widgets and GPU previews belong in `serein-studio`. Avoid leaking widget types into domain models or Win32 handles across pure interfaces. Add a production player crate only with its actual player slice.

Use typed messages and explicit state transitions. File operations must preserve newer edits and reject unknown schemas. Never silently replace dirty work. Keep previous valid output on shader errors. Distinguish generated data, measured data, user settings and planned features in both code and UI.

## Style

Run rustfmt. Name units in APIs, e.g. `gain_db` and `attack_ms`. Comments should explain why a choice exists or which lifetime/format guarantee it relies on. Every unsafe block needs a safety argument. User/device/file errors need a recovery path; reserve assertions for tested program invariants.

Keep UI tokens and component styles in `theme.rs`; common labelled controls live in `views.rs`. Split views into their own modules as their behavior grows. Don't duplicate a distinct visual convention just to build one screen. Keep standard controls for editable values and a keyboard alternative for canvas operations.

## Verification

Run the README checks. Test changes where actual behavior matters: DSP frequency response, signal silence/release, malformed imports, OS-adapter recovery and unsaved work. A rendering change needs inspection in the native app, including a narrow window. Compilation does not prove layout or accessibility.

Changes to runtime architecture, package format, permissions, performance targets or SDK compatibility need a short decision note in `docs/`. Record the workload and hardware with benchmark claims. Keep secrets, captured audio, private titles and input histories out of fixtures and diagnostics.

Before external contribution/distribution, agree the repository license; none has been selected by the owner yet.
