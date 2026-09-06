# Changelog

## 2026-09-06
- fix(ci): aligned desktop verification with the release-tag workflow, pinned
  Flutter and GitHub Actions dependencies, and rejected incomplete runtime bundles.
- fix(ci): corrected the pinned Flutter action commit so release jobs can
  resolve the setup action.
- fix(ci): compiled the Windows setup launcher with the .NET Framework
  compiler for PowerShell 7 runners and matched Linux verification to Flutter's
  actual release bundle layout.
- fix(ci): checked out the repository in the release publisher so
  `gh release create --verify-tag` can resolve the pushed tag.
- feat(release): added a self-extracting Windows setup executable alongside
  the portable ZIP release.
- fix(release): changed the Windows setup asset into a GUI installer wizard
  without a console window.
- fix(release): replaced the unsigned custom setup EXE with a script-based GUI
  setup ZIP to avoid antivirus quarantine of the self-extracting binary.
- fix(release): bundled Windows FFmpeg in the setup payload and reused
  installed local binaries before downloading replacements.
- fix(release): documented targeted branch-and-tag pushes so existing local
  tags were not retried during publication.
- feat(spull): migrated the desktop downloader to Dart and Flutter.
- fix(media): embedded high-quality center-cropped square album art into audio exports without sidecar images.
- refactor(ui): removed mascot-themed copy and decorative blocks, then clarified the light logo-derived dashboard hierarchy.
- fix(download): escaped thumbnail filter commas so yt-dlp passes the square crop to ffmpeg reliably.
- refactor(ui): softened the light palette and reduced pixel shadows to avoid eye strain while preserving logo accents.
- feat(branding): added a transparent pixel logo and matching Windows/macOS app icons.
- chore(packaging): configured Windows and macOS Flutter release packaging, including macOS ad-hoc signing.
- feat(linux): added the GTK desktop target and a Linux x86_64 release artifact to CI.
- fix(linux): added automatic yt-dlp, ffmpeg, and Deno bootstrap paths for Linux.
- refactor(folder): defaulted fresh installs to Downloads and made folder selection reopen at the current path.
- refactor(ui): moved download-folder setup into a dedicated destination panel with clear change and open actions.
- refactor(branding): replaced fluorescent mint accents in the UI and raster app icons with a muted sage tone.
- ci(release): removed the Intel macOS matrix job and retained Apple Silicon, Windows, and Linux artifacts.
- fix(ci): packaged the complete Windows release bundle with `data/` and `flutter_windows.dll` instead of publishing only the executable.
- feat(sites): loaded the complete supported-extractor catalog from yt-dlp and preserved current-broken markers.
