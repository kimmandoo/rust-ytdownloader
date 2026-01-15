# YouTube Downloader built with Rust

A modern, cross-platform YouTube downloader built with Rust and egui.

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Build Status](https://github.com/kimmandoo/rust-ytdownloader/actions/workflows/release.yml/badge.svg)

## 🚀 Features

- **Modern GUI**: Clean and responsive user interface built with `egui`.
- **Cross-Platform**: Works on Linux, Windows, and macOS.
- **Auto-Setup**: Automatically downloads necessary dependencies (`yt-dlp`, `ffmpeg`) on first run. No complex manual setup required.
- **Multiple Formats**:
  - Audio: MP3, WAV, M4A, FLAC
  - Video: MP4, WEBM
- **Playlist Support**: Download entire playlists or select specific videos.
- **Metadata Embedding**: Automatically adds thumbnails and metadata to downloaded files.

## 📦 Installation

Download the latest release for your platform from the [Releases Page](https://github.com/kimmandoo/rust-ytdownloader/releases).

### Supported Platforms
- **Windows**: `x86_64`
- **Linux**: `x86_64`
- **macOS**: `Intel` & `Apple Silicon`

## 🛠️ Usage

1. Run the application.
2. On first launch, wait for the initialization to complete (downloads `yt-dlp` and `ffmpeg`).
3. Select a download folder.
4. Paste a YouTube URL and click **Analyze**.
5. Select the format and quality, then click **Download**.

## ⚠️ Legal Disclaimer & Terms of Service

**Please read this section carefully before using this software.**

This project is for **educational purposes only**. The developers of this software functionality do not endorse or encourage any potential violation of YouTube's Terms of Service or copyright laws.

1. **Personal Use Only**: This tool is intended strictly for personal, private use (e.g., time-shifting, formatting shifting). Any commercial use or redistribution of downloaded content is strictly prohibited.
2. **Copyright Compliance**: Users are responsible for ensuring that they have the right to download any content. Do not download copyrighted material without permission from the copyright holder.
3. **YouTube Terms of Service**: Downloading content from YouTube may violate their [Terms of Service](https://www.youtube.com/t/terms). Specifically, the ToS may prohibit downloading content unless a download button or similar link is displayed by YouTube on the Service for that Content.
4. **No Liability**: The authors and contributors of this project are not liable for any misuse of this software or any legal consequences arising from its use. The user assumes full responsibility for their actions.

---

## 🏗️ Development

### Prerequisites
- Rust (latest stable)
- `libssl-dev`, `pkg-config` (Linux)
- `cmake` (System)

### Build
```bash
cargo run
```

### Build for Release
```bash
cargo build --release
cargo build --release --target x86_64-pc-windows-gnu # for windows
```

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
