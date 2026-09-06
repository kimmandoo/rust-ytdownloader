# Work checkpoint

- Active task: Reduced the Windows setup first-launch delay by bundling the
  FFmpeg executable in the setup runtime and making the backend reuse installed
  binaries.
- Next action: Commit this completed change; no blockers remain.
- Changed files: `lib/services/spull_backend.dart`,
  `tool/package_windows_release.ps1`, `README.md`, `CHANGELOG.md`, and this
  checkpoint.
- Runtime behavior: The setup payload now contains `bin/ffmpeg.exe`. Spull
  checks `%LOCALAPPDATA%/Spull/bin`, then `bin/` beside the app executable, then
  `PATH`; an absent or unusable FFmpeg is downloaded once into the persistent
  app-data bin directory.
- Verification:
  - `dart run tool/verify.dart` passed: formatting, Flutter analyzer, and all
    widget tests passed.
  - `dart run tool/verify_desktop_artifact.dart windows` passed.
  - Windows PowerShell parser accepted `tool/package_windows_release.ps1`.
  - Local Windows packaging passed and produced portable plus setup ZIPs.
  - Nested `spull-runtime.zip` contains `bin/ffmpeg.exe` (102,856,192 bytes
    uncompressed); portable ZIP remains FFmpeg-free.
  - `git diff --check` passed.
- Blockers: None.
