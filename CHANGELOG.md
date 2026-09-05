# Changelog

## 2026-09-06

- feat(spull): migrated the desktop downloader to Dart and Flutter.
- fix(media): embedded high-quality center-cropped square album art into audio exports without sidecar images.
- refactor(ui): removed mascot-themed copy and decorative blocks, then clarified the light logo-derived dashboard hierarchy.
- fix(download): escaped thumbnail filter commas so yt-dlp passes the square crop to ffmpeg reliably.
- refactor(ui): softened the light palette and reduced pixel shadows to avoid eye strain while preserving logo accents.
- feat(branding): added a transparent pixel logo and matching Windows/macOS app icons.
- chore(packaging): configured Windows and macOS Flutter release packaging, including macOS ad-hoc signing.
