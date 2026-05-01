# Spull

Spull is a desktop media downloader for Windows and macOS with a Rust core and a Tauri UI.

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Build Status](https://github.com/kimmandoo/rust-ytdownloader/actions/workflows/release.yml/badge.svg)

## Features

- Tauri desktop app with a polished, scrollable WebView UI.
- Rust download core using `yt-dlp`, `ffmpeg`, and Deno setup.
- Stable support for YouTube URLs, with experimental support for other `yt-dlp`-compatible media sites.
- Audio formats: MP3, WAV, M4A, FLAC.
- Video formats: MP4, WEBM.
- Playlist analysis with per-video selection.
- Visible progress, recent logs, stop control, and download folder shortcuts.

## Development

### Prerequisites

- Rust stable
- Node.js and npm
- Windows: Visual Studio Build Tools with MSVC and Windows SDK
- macOS: Xcode Command Line Tools

### Run the Tauri App

```bash
npm install
npm run tauri dev
```

### Build the Tauri App

```bash
npm run tauri build
```

The Windows build produces a native `.exe` and installer artifacts through Tauri.

### Frontend Only

```bash
npm run dev
npm run build
```

The frontend dev server runs at `http://127.0.0.1:1420`.

## Legal Disclaimer

This project is for educational and personal use only. Users are responsible for complying with each source site's terms of service and copyright law. Do not download copyrighted material unless you have the right to do so.

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE) for details.
