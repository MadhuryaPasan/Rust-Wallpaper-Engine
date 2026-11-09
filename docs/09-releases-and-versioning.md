# Releases and version numbers

## One version source

Edit `version` under `[workspace.package]` in the root **Cargo.toml**. All six
crates use `version.workspace = true`; do not add individual crate versions.

| Location | How it receives the version |
| --- | --- |
| Root `Cargo.toml`, `[workspace.package].version` | Edit this value manually |
| `Cargo.lock`, six `serein-*` package entries | Run `cargo check --workspace` after changing the root version; commit the updated lockfile |
| Studio's sidebar | `CARGO_PKG_VERSION`, supplied by Cargo |
| Both executables' Windows File/Product version | `scripts/windows-resource.rs`, included by each executable's `build.rs` |
| Installer AppVersion and Windows numeric version | `scripts/package.ps1` passes values to `installer/serein.iss` |
| Installer, portable ZIP and artifact folder names | `scripts/package.ps1` reads Cargo metadata |
| GitHub release name and asset paths | `.github/workflows/release.yml` reads the same metadata |
| Git tag | Create `v` followed by that exact version |

The installer AppId is a permanent product identity, **not** a version number.
Keep it stable for upgrades. Likewise, project `schema_version` and shader SDK
versions describe compatibility; do not change them for ordinary app releases.
The pinned Inno Setup version in `setup-inno.ps1` and its discovery path in
`package.ps1` are build-tool versions, separate from Serein's version.

## Create a release

For example, to release 0.2.0:

1. Set `[workspace.package].version = "0.2.0"` in root `Cargo.toml`.
2. Run `cargo check --workspace` to update Cargo.lock.
3. Run formatting, tests and clippy from README. Optionally build the installer
   locally with `.\scripts\package.ps1 -Tag v0.2.0`.
4. Commit your source and version changes, including Cargo.lock and the workflow.
5. Push the commit, then create and push the matching tag:

```powershell
git tag -a v0.2.0 -m "Serein 0.2.0"
git push origin HEAD
git push origin v0.2.0
```

The repository must already be hosted on GitHub with the `origin` remote set.
This workspace currently has no remote configured; the workflow cannot run until
you push the repository to GitHub. No tag or release has been published locally
by adding these files.

The workflow runs for pushed `v*` tags, verifies the tag against Cargo, runs
checks, builds both EXEs, packages them and publishes a GitHub release with:

- `Serein-<version>-windows-x64-setup.exe`
- `Serein-<version>-windows-x64-portable.zip`
- `SHA256SUMS.txt`

It uses the built-in GitHub token with job-level `contents: write`; no personal
access token is required. Repository or organization policy must allow this
permission. Checks and packaging must succeed before publication. Uploads are
completed on a draft before it becomes public. Reruns replace assets for the same
tag; don't move a released tag to different source.

The Actions **Run workflow** button can test a branch manually. Manual runs
upload downloadable Actions artifacts without publishing a GitHub release.

## Supported versions

Use SemVer `MAJOR.MINOR.PATCH`, such as `0.2.0`. A prerelease such as
`0.2.0-beta.1` is also supported and is marked as a prerelease on GitHub.
Build metadata (`+...`) is not supported for release packaging. Windows file
version fields use `MAJOR.MINOR.PATCH.0`; each component must fit in 16 bits.
The full version, including a prerelease suffix, remains in package names and
the textual product version.

A tag such as `v0.2.1` with Cargo still at `0.2.0` fails before packaging.
To validate locally:

```powershell
. .\scripts\version.ps1
Get-SereinVersion -ExpectedTag v0.2.0
```

## Installer behavior

The x64 installer defaults to `%LOCALAPPDATA%\Programs\Serein`, creates Start-menu
entries and registers an uninstaller. It offers an optional desktop shortcut.
It bundles both the studio and desktop player, README and documentation. Built-in
shader sources, preview images and the creator guide are embedded in the binaries.
No Rust toolchain is needed by the person installing Serein.

The Microsoft VC runtime DLLs come from Visual Studio's x64 redistributable
directory and are included beside both executables. The portable ZIP includes
them too. Packaging fails if the redistributable files cannot be found.

CI also installs and uninstalls the package in a temporary folder, checks both
executable product versions and validates the uninstall registration. You can run
the same check with `scripts/test-installer.ps1 -Installer <path-to-setup.exe>` in
an account without an existing installed Serein. This is a packaging test; it does
not visually qualify desktop playback on every Windows build.

Exit Serein from the tray before upgrading or uninstalling. Closing its window
only hides it. Setup uses Windows Restart Manager to detect files in use and can
ask to close the application. It does not force-kill it or automatically restart
old wallpaper sessions. User data under `%LOCALAPPDATA%\Serein` is preserved.
Deleting that folder manually removes imported images, drafts and settings.

Packages are currently unsigned. SHA-256 checksums verify downloaded file
integrity but are not publisher authentication. There is no automatic updater
or startup registration in this installer.
