# Spull

<p align="center">
  <img src="assets/spull_logo.svg" alt="Spull transparent pixel logo" width="128" />
</p>

<p align="center"><strong>SPULL // MEDIA DOWNLOADER</strong><br />A clean 2D pixel-game desktop media downloader built entirely with Dart and Flutter.</p>

![License](https://img.shields.io/badge/license-MIT-blue.svg)

## Features

- Flutter desktop UI for Windows and macOS with a 2D pixel-game visual language.
- Dart backend that runs yt-dlp and ffmpeg as managed desktop processes.
- Stable YouTube support plus experimental yt-dlp-compatible media sites.
- Audio formats: MP3, WAV, M4A, FLAC. Video formats: MP4, WEBM.
- Playlist analysis with per-item selection, live progress, logs, stop control, and output-folder shortcuts.
- MP3 exports embed a high-quality center-cropped square album cover instead of leaving a sidecar image.
- Long transfers keep resumable fragments, retry network fragments, trim path lengths, throttle UI events, and terminate child processes on cancel.
- Browser-cookie and `cookies.txt` authentication options.
- A transparent pixel logo and matching macOS/Windows app icons.

## Development

### Prerequisites

- Flutter stable with Dart 3.13 or newer.
- Internet access on first launch so Spull can automatically install yt-dlp, ffmpeg, and the optional Deno runtime.
- Existing `yt-dlp` and `ffmpeg` binaries on `PATH` are detected and reused.
- macOS: Xcode Command Line Tools for building the macOS target.
- Windows: Visual Studio Build Tools with the Desktop C++ workload.

### Run the app

```bash
flutter pub get
flutter run -d windows
# or
flutter run -d macos
```

### Build releases

```bash
flutter build windows --release
flutter build macos --release
```

The macOS Release configuration uses `CODE_SIGN_IDENTITY = -` and disables provisioning-profile requirements, producing an ad-hoc signed app suitable for local distribution. It is not notarized and cannot be used as a Mac App Store submission. See [`macos/ExportOptions.plist`](macos/ExportOptions.plist).

### Dependency locations

Spull checks app-local binaries first and then falls back to the system `PATH`:

- Windows: `%LOCALAPPDATA%/Spull/bin`
- macOS: `~/Library/Application Support/Spull/bin`

Settings are stored as JSON in the platform's Spull application-data folder.

## Legal Disclaimer

This project is for educational and personal use only. Users are responsible for complying with each source site's terms of service and copyright law. Do not download copyrighted material unless you have the right to do so.

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE) for details.
