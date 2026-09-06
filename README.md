# Spull

<p align="center">
  <img src="assets/spull_logo.svg" alt="Spull transparent pixel logo" width="128" />
</p>

<p align="center"><strong>SPULL // MEDIA DOWNLOADER</strong><br />A clean 2D pixel desktop media downloader built entirely with Dart and Flutter.</p>

![License](https://img.shields.io/badge/license-MIT-blue.svg)

## Features

- Flutter desktop UI for Windows, macOS, and Linux with a 2D pixel visual language.
- Dart backend that runs yt-dlp and ffmpeg as managed desktop processes.
- Stable YouTube support plus experimental yt-dlp-compatible media sites.
- Audio formats: MP3, WAV, M4A, FLAC. Video formats: MP4, WEBM.
- Playlist analysis with per-item selection, live progress, logs, stop control, and output-folder shortcuts.
- Live supported-extractor catalog loaded from the installed yt-dlp binary, with search and current-broken markers.
- MP3 exports embed a high-quality center-cropped square album cover instead of leaving a sidecar image.
- Long transfers keep resumable fragments, retry network fragments, trim path lengths, throttle UI events, and terminate child processes on cancel.
- Browser-cookie and `cookies.txt` authentication options.
- A transparent pixel logo and matching desktop app icons.

## Development

### Prerequisites

- Flutter stable with Dart 3.13 or newer.
- Internet access on first launch so Spull can automatically install yt-dlp, ffmpeg, and the optional Deno runtime.
- Existing `yt-dlp` and `ffmpeg` binaries on `PATH` are detected and reused.
- Linux: `clang`, `cmake`, `ninja-build`, `pkg-config`, `libgtk-3-dev`, and `libstdc++-12-dev`.
- macOS: Xcode Command Line Tools for building the macOS target.
- Windows: Visual Studio Build Tools with the Desktop C++ workload.

### Run the app

```bash
flutter pub get
flutter run -d windows
# or
flutter run -d macos
# or
flutter run -d linux
```

### Build releases

```bash
flutter build windows --release
flutter build macos --release
flutter build linux --release
```

The Linux release artifact is a `.tar.gz` containing the complete Flutter bundle
at its root; extract it and run `spull`.
Windows releases provide two options:
- `spull-windows-x86_64.zip`: portable bundle; extract the full archive and run
  `spull.exe`.
- `spull-windows-x86_64-setup.zip`: antivirus-safe GUI installer bundle;
  extract it and double-click `Install-Spull.vbs`. The setup payload includes
  Windows FFmpeg, so the first launch does not wait for an FFmpeg download.
  The wizard installs Spull under `%LOCALAPPDATA%\Spull`, creates a Start Menu
  shortcut, and can launch the app.

The macOS Release configuration uses `CODE_SIGN_IDENTITY = -` and disables provisioning-profile requirements, producing an ad-hoc signed app suitable for local distribution. It is not notarized and cannot be used as a Mac App Store submission. See [`macos/ExportOptions.plist`](macos/ExportOptions.plist).

### Continuous integration

GitHub Actions verifies and builds Linux, macOS, and Windows on every
`release-*` tag. A release tag must point to a commit whose subject starts with
`release(scope):`, such as `release(v1.2.0): publish desktop artifacts`.
To publish one release without retrying every old local tag, push only the
branch and release tag:

```bash
git push origin main release-v1.2.1
```

Do not use `git push origin main --tags`; it attempts to push every local tag,
including tags that already exist on the remote. `workflow_dispatch` runs the
same verification and uploads complete desktop bundles without publishing a
release. Ordinary branch pushes do not publish artifacts.

The CI build validates the platform executable together with its Flutter
runtime files before packaging. Windows archives place `spull.exe`,
`flutter_windows.dll`, plugin DLLs, and `data/` at the archive root; extract the
whole archive before launching the app. The setup archive additionally places
`ffmpeg.exe` under `bin/` in the installed runtime.

The Windows release job publishes the portable ZIP and a script-based GUI setup
bundle instead of an unsigned custom setup executable.

### Dependency locations

Spull checks binaries in this order:

1. `%LOCALAPPDATA%/Spull/bin` (downloaded tools; retained between launches)
2. `bin/` beside the installed `spull.exe` (including FFmpeg from the setup ZIP)
3. The system `PATH`

If FFmpeg is absent or unusable, Spull downloads it once into the first
location and reuses that copy on later launches. Settings are stored as JSON in
the platform's Spull application-data folder.

## Legal Disclaimer

This project is for educational and personal use only. Users are responsible for complying with each source site's terms of service and copyright law. Do not download copyrighted material unless you have the right to do so.

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE) for details.
