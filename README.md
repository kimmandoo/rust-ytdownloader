# Spull

<p align="center">
  <img src="assets/spull_logo.svg" alt="Spull pixel logo" width="144" />
</p>

<p align="center">
  <strong>SPULL // MEDIA DOWNLOADER</strong><br />
  A focused desktop media downloader powered by Flutter, Dart, and yt-dlp.
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-2A2037.svg" alt="MIT license" /></a>
  <img src="https://img.shields.io/badge/Flutter-desktop-4F7565.svg" alt="Flutter desktop" />
  <img src="https://img.shields.io/badge/Dart-3.13%2B-E28A66.svg" alt="Dart 3.13 or newer" />
</p>

Spull turns one or more links into a selectable download queue. It keeps the interface quiet and predictable: inspect first, choose what to keep, then download to a folder you control.

## What it does

| Area | Behavior |
| --- | --- |
| **Inspect** | Analyzes single videos and playlists with yt-dlp JSON output. Each link has a 90-second process deadline. |
| **Select** | Shows playlist entries with thumbnails, durations, source labels, and per-item selection. |
| **Download** | Runs a sequential queue with live yt-dlp output, elapsed time, ETA, retry support, and cancel control. |
| **Formats** | MP3, WAV, M4A, FLAC, MP4, and WEBM. Audio exports receive square, center-cropped album art. |
| **Compatibility** | Loads yt-dlp's extractor catalog with search and `CURRENTLY BROKEN` markers. Catalog lookup has a 30-second deadline and cancel control. |
| **Authentication** | Uses a browser cookie profile or a selected `cookies.txt` file when a source requires login. |
| **Bootstrap** | Reuses local tools when possible. Downloads report speed and remaining time; servers without `Content-Length` use an indeterminate progress bar instead of a fake percentage. |

> Use Spull only for media you are allowed to download. yt-dlp support depends on the source site and its current policies.

## Screenshots in one sentence

Light logo-derived colors, compact pixel accents, no decorative clutter: the queue and its current state remain the visual priority.

## Quick start

### Requirements

- Flutter stable with Dart 3.13 or newer.
- Internet access on first launch if yt-dlp, FFmpeg, or optional Deno must be installed.
- Existing `yt-dlp` and `ffmpeg` binaries on `PATH` are detected and reused.
- **Windows:** Visual Studio Build Tools with the Desktop C++ workload.
- **macOS:** Xcode Command Line Tools.
- **Linux:** `clang`, `cmake`, `ninja-build`, `pkg-config`, `libgtk-3-dev`, and `libstdc++-12-dev`.

### Run locally

```bash
flutter pub get
flutter run -d windows
# or
flutter run -d macos
# or
flutter run -d linux
```

### Build locally

```bash
flutter build windows --release
flutter build macos --release
flutter build linux --release
```

The complete Flutter runtime bundle is required at launch. Do not copy only `spull.exe` or only the Linux executable.

## Dependency resolution

Spull checks executable locations in this order:

1. `%LOCALAPPDATA%/Spull/bin` on Windows, or the platform Spull data directory on macOS/Linux.
2. `bin/` beside the installed `spull` executable.
3. The system `PATH`.

The first-launch bootstrap can install:

- **yt-dlp** — required for analysis and downloads.
- **FFmpeg** — required for audio extraction and media conversion.
- **Deno** — optional; improves extractor compatibility.

Bootstrap status includes the current tool, progress mode, transfer speed, and ETA when a total size is available. A partial download is kept separate until it completes, so an interrupted tool download does not replace a working binary.

## Release artifacts

### Windows

The release workflow produces two archives:

- `spull-windows-x86_64.zip` — portable Flutter bundle. Extract the whole archive and run `spull.exe`.
- `spull-windows-x86_64-setup.zip` — antivirus-safe GUI setup bundle. Extract it and double-click `Install-Spull.vbs`.

The setup bundle contains Windows FFmpeg under `bin/`, so a fresh setup does not wait for the large FFmpeg archive on first launch. The installer defaults to `%LOCALAPPDATA%\\Spull`, creates Start Menu shortcuts for Spull and `Uninstall Spull`, and can launch the app after installation.

For removal:

- Use the **Uninstall Spull** Start Menu shortcut from an installed copy.
- The setup folder also contains `Uninstall-Spull.vbs` for the default install path.
- For a custom install directory, run the uninstaller copy inside that installed directory.

### Linux

The Linux artifact is a `.tar.gz` containing the complete Flutter bundle at its root. Extract it and run `spull`.

### macOS

The macOS release uses ad-hoc signing (`CODE_SIGN_IDENTITY = -`) without provisioning profiles. It is suitable for local distribution, but it is not notarized and is not a Mac App Store submission. See [`macos/ExportOptions.plist`](macos/ExportOptions.plist).

## Publishing a release

Feature work must be committed first. The guarded publisher updates the patch release build number, verifies the repository, creates the release commit and annotated tag, then pushes only the requested branch and tag.

```powershell
powershell -NoProfile -ExecutionPolicy Bypass `
  -File tool/publish_release.ps1 -Version 1.2.4
```

The release tag must point to a commit whose subject starts with `release(scope):`, for example:

```text
release(v1.2.4): publish desktop artifacts
```

The script intentionally performs a targeted push equivalent to:

```bash
git push origin main release-v1.2.4
```

Do **not** use `git push origin main --tags`; that retries every local tag, including tags already present on the remote. GitHub Actions builds Linux, macOS, and Windows only for `release-*` tags. `workflow_dispatch` remains available for manual verification without publishing a release.

## Project map

```text
lib/
├── home_page.dart                 Flutter dashboard and controls
├── models/media_models.dart       Settings, queue, and event models
├── services/spull_backend.dart    yt-dlp, FFmpeg, downloads, bootstrap
└── state/app_controller.dart      UI state, cancellation, and queue flow

tool/
├── verify.dart                    Local/CI formatter, analyzer, and test gate
├── publish_release.ps1            Guarded branch/tag release publisher
├── package_windows_release.ps1   Portable and setup archive packaging
├── Install-Spull.ps1              Windows setup wizard
└── Uninstall-Spull.ps1            Windows removal script
```

## Development checks

Run the same gate used by CI:

```bash
dart run tool/verify.dart
```

This runs dependency resolution, formatting checks, Flutter analysis, and the widget test suite.

## Legal

Spull is for educational and personal use. You are responsible for complying with each source site's terms of service, copyright law, and any authentication requirements. Do not download copyrighted material unless you have the right to do so.

## License

MIT. See [LICENSE](LICENSE).
