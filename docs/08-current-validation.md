# Current validation

## Preview color correction — 2026-11-09

The user's newer screenshot confirms the desktop layer fix is visible. It also
shows a separate color mismatch: Iced's default `web-colors` mode uses a plain
UNORM render target, whereas the desktop and thumbnail renderers use sRGB targets.
The shared shader previously returned linear RGB directly to both, so only the
sRGB targets performed display encoding. The preview was too dark.

The shared material now specializes the fragment shader for the target format.
Plain UNORM targets explicitly encode linear RGB to sRGB; sRGB targets use the
hardware encoding. The desktop's colors are retained; the live canvas now follows
the same color contract. This is a transfer-function correction, not a brightness
or contrast adjustment. No per-frame readback is used for playback.

The explicit GPU regression test renders a known dark/midtone patch into both
formats and checks the actual bytes against expected sRGB values. It also compares
all ten presets and checks that a known imported sRGB image retains its pixel
colors through sampling and output. It passed on the development GPU:

```powershell
cargo test -p serein-render --test color_output -- --ignored
```

The test is opt-in because hosted CI runners may have no graphics adapter.
HDR and per-monitor color-profile behavior still need separate qualification.

## Desktop attachment correction — 2026-11-09

The earlier report of successful desktop playback was based on submitted frames.
The user's screenshot demonstrated that this was insufficient: Windows' original
wallpaper was still visible. Treat that earlier visual-success claim as invalid.

The shell on the development machine has `WS_EX_NOREDIRECTIONBITMAP` on Progman,
a layered `SHELLDLL_DefView`, and a WorkerW child containing the static background.
The player now attaches its own layered child directly to Progman, below the icons
and above WorkerW. Legacy desktops retain the separate WorkerW hosting path.

Presentation uses wgpu 27's DX12 DirectComposition visual instead of an HWND flip
swapchain. Frames stay on the GPU; there is no per-frame CPU image readback.
Attachment happens after winit shows the window, because showing it reapplies
winit's cached native styles. Doing this earlier silently removed the required
child/layered styles.

The player checks parentage, visible flags, layered style, sibling ordering and
physical monitor bounds once per second. Lost attachment triggers recreation.
Destroy events from replaced surfaces are ignored. Studio also clears a stale
running status when the child process exits.

Read-only shell inspection:

```powershell
cargo run -p serein-platform --example desktop_diagnostics
```

On this machine the corrected surface was observed between DefView and WorkerW,
with extended style `0x08080080`, physical bounds 1920 × 1080 at (0, 0), and an
advancing frame count. This verifies native attachment and presentation setup.
It does **not** establish the final visible desktop pixels. The available UI
capture tool exposes application windows, not the desktop; final desktop visual
confirmation remains pending.

## Automated checks

- `cargo test --workspace`: 22 passed; one hardware audio test intentionally ignored.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- Optimized workspace build: builds both executables for distribution together.

These checks do not qualify all Windows builds, HDR, physical multi-monitor
arrangements, Explorer restart, docking or third-party desktop extensions.

## Existing feature verification

Automatic audio capture was previously observed responding to browser playback,
using Default output and the named Realtek endpoint. A missing pinned device
automatically retries in a test. Physical device hotplug remains unverified.
All ten bundled shaders and the creator guide's WGSL examples compile in tests.
Image depth is an approximation unless the user supplies a depth map.

Windows desktop integration references:

- [Lively desktop compatibility discussion, including Microsoft's layered-window explanation](https://github.com/lively-community/lively/issues/2074)
- [wgpu DX12 presentation options](https://docs.rs/wgpu-types/27.0.1/wgpu_types/enum.Dx12SwapchainKind.html)
- [Microsoft layered window documentation](https://learn.microsoft.com/en-us/windows/win32/winmsg/window-features#layered-windows)
