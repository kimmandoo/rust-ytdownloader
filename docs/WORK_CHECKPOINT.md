# Work checkpoint

- Active task: Prepared release `v1.2.1` for the bundled Windows FFmpeg setup.
- Next action: Push this commit and the `release-v1.2.1` tag to trigger the
  release workflow.
- Changed files: `pubspec.yaml`, `docs/WORK_CHECKPOINT.md`.
- Release contents: Windows setup includes `bin/ffmpeg.exe`; Spull reuses
  installed FFmpeg before falling back to download or `PATH`.
- Verification already passed:
  - `dart run tool/verify.dart`: formatting, Flutter analyzer, and widget tests.
  - `dart run tool/verify_desktop_artifact.dart windows`.
  - PowerShell parser accepted `tool/package_windows_release.ps1`.
  - Local Windows packaging produced portable and setup ZIPs.
  - Nested `spull-runtime.zip` contained `bin/ffmpeg.exe`; portable ZIP stayed
    FFmpeg-free.
  - `git diff --check`.
- Blockers: None. The release tag and push remain intentionally unperformed.
