# Serein

A Windows wallpaper app written in Rust, with a visual library, automatic audio response, optional shader tools and a separate desktop player.

## Install and run

On GitHub, open this repository's **Releases** page and download `Serein-<version>-windows-x64-setup.exe`. Run it, then open **Serein** from the Start menu. The installer installs for your Windows account without requiring administrator access. An optional desktop shortcut is available. Use **Settings > Apps > Installed apps > Serein** to uninstall. Your imported images and settings remain in `%LOCALAPPDATA%\Serein`.

For a portable copy, extract `Serein-<version>-windows-x64-portable.zip` and run `serein-studio.exe`. Keep the complete extracted folder together. Both forms require 64-bit Windows 10 (1809+) or Windows 11 and a DirectX 12 capable GPU/driver. Windows 11 is the currently exercised desktop-hosting path. Release files are unsigned at present.

## Build locally

Install [Rust via rustup](https://rustup.rs/) with the MSVC toolchain, and [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022) with **Desktop development with C++**, the MSVC compiler and a Windows SDK. Git is needed to clone the repository. Use current stable Rust; `rust-version` in Cargo.toml is a declared lower bound, not a verified minimum for every locked dependency.

Open PowerShell in the project folder:

```powershell
cargo build --release --workspace --locked
.\target\release\serein-studio.exe
```

Keep `serein-studio.exe` and `serein-player.exe` together. Use the optimized build for everyday playback. Rust and the Visual Studio C++ build tools are needed for a source build. No browser, account or cloud service is required for native wallpapers.

The equivalent helper is `.\scripts\build.ps1`. For a debug build, run `cargo build --workspace --locked` and `.\target\debug\serein-studio.exe`. Build the whole workspace so the desktop player is available. Exit the corresponding running build from the tray before rebuilding it; Windows locks running executable files.

## Build an installer locally

```powershell
rustup target add x86_64-pc-windows-msvc
.\scripts\setup-inno.ps1
.\scripts\package.ps1
```

The first script downloads and verifies the signed Inno Setup 7.1.1 compiler, then installs it in `%LOCALAPPDATA%\SereinBuildTools`. Alternatively install Inno Setup yourself and supply `-IsccPath 'C:\path\to\ISCC.exe'` to `package.ps1`. Check [Inno Setup's licensing terms](https://jrsoftware.org/isinfo.php) for your use.

Packaging builds both executables for Windows x64 with the lockfile, and writes an installer, portable ZIP and `SHA256SUMS.txt` under `artifacts/<version>/`. Shader assets and previews are embedded; no extra runtime asset download is needed. See the [release and version guide](docs/09-releases-and-versioning.md) for tag-triggered GitHub builds and every version location.

## Apply a wallpaper

1. Select a visual tile in **Library**.
2. Open **Customize** for brightness, speed, scene zoom, color, audio response and mouse depth. Code is optional, under **Advanced**.
3. Choose Primary, Selected, Every display, or Span and select **Apply wallpaper**.
4. Close or minimize the window. Serein moves to the system tray while the wallpaper continues.

The tray menu reopens the app, pauses/resumes or stops playback, and exits. Stop or Exit reveals your original Windows background. Serein does not overwrite the Windows wallpaper setting.

## Automatic audio

The equalizer works automatically. There is no Start/Stop analyzer control.

**Audio lab > Default output** follows Windows device changes. A named output stays pinned and reconnects automatically after disconnection. This selection persists locally. The studio listens while visible; the separate player handles audio for an active desktop wallpaper while the studio is hidden. Playback pause suspends capture. Zero audio reaction releases the player's audio capture.

EQ shapes a copy of output audio for the effect; it does not change speaker playback, open a microphone, save recordings or transmit audio.

## Wallpapers and image depth

Ten native presets are included: Chromatic current, Orbital form, Contour field, Liquid chrome, Dune horizon, Deep space, Ocean dusk, Neon lattice, Aurora veil and Prism bloom. Thumbnails are rendered from the actual shaders. The 3D forms are shader geometry, not a full mesh editor.

**Import an image** adds a mouse-responsive 2.5D wallpaper to **Your wallpapers**. Images are copied locally and small thumbnails are generated. Supply a depth map or use the local compositional/luminance approximation. This is not AI depth estimation or geometry reconstruction. Settled photo wallpapers avoid submitting unchanged GPU frames.

## Create effects

Read the [creator guide](docs/07-creator-guide.md), also available from **Advanced > Read the creator guide**. It includes compiling examples for audio response, reusable WGSL functions, 3D shapes and image depth.

`Ctrl+Enter` compiles; `Ctrl+S` exports; `Ctrl+Z`/`Ctrl+Y` undo/redo code. Invalid source retains the last good output. `Ctrl+K` opens commands including `apply`, `customize`, `device default`, `frequency 120`, and `help`.

Settings, preset profiles, imported-image library and recovery drafts live under `%LOCALAPPDATA%\Serein`. Exported JSON contains source/settings and local image references; a portable asset archive is not implemented yet.

## Architecture and checks

The workspace separates domain models, audio, shared rendering, Windows adapters, desktop player and studio into six crates. See [architecture](docs/03-architecture.md), [research](docs/01-research.md), [product plan](docs/02-product-plan.md), [current validation](docs/08-current-validation.md) and [contributing](CONTRIBUTING.md).

```powershell
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
python scripts/check_contrast.py
```

Hardware loopback smoke test: `cargo test -p serein-audio -- --ignored`.

## Current limits

This is a development build. Video/web players, full mesh/node/timeline editing, code signing/startup, community packages and full screen-reader support remain future work. Explorer desktop hosting is an undocumented shell convention; OS updates can require compatibility work. Display modes are implemented, but the complete physical multi-monitor, docking and Explorer-restart matrix needs qualification. No CPU/RAM/GPU/energy superiority benchmark has been completed.

Hiding to the tray suspends studio preview/capture but retains UI memory. The desktop player is separate. No project distribution license has been selected yet.
