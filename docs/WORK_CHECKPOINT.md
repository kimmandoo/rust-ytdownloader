# Work checkpoint

- Active task: Added a self-extracting Windows setup executable alongside the
  portable release archive.
- Next action: Create the release commit, tag it as `release-v1.2.0`, and push
  the commit plus tag.
- Changed files: `.github/workflows/release.yml`,
  `tool/package_windows_release.ps1`, README, changelog, and this checkpoint.
- CI result: GitHub Actions now runs only for `release-*` tags or manual
  dispatch, validates `release(scope):` commit subjects, pins Flutter/action
  dependencies, verifies all three desktop bundles before packaging, and
  publishes four assets after all matrix builds pass: Linux tarball, macOS app
  archive, Windows portable ZIP, and Windows setup executable.
- Catalog result: The support panel runs `yt-dlp --list-extractors`, displays
  every returned extractor in a scrollable list, supports case-insensitive
  search, and carries `(CURRENTLY BROKEN)` markers into the UI status.
- Verification:
  - Live extractor smoke loaded 1,751 entries from the installed yt-dlp binary
    and preserved 136 broken statuses.
  - `dart run tool/verify.dart` passed: formatting, Flutter analyzer, and all
    widget tests passed.
  - A clean Windows release build passed, then
    `dart run tool/verify_desktop_artifact.dart windows` found the executable,
    Flutter DLL, engine data, and asset manifest.
  - The Windows package script created the portable ZIP and setup EXE.
  - Both archives' runtime payloads were inspected; the setup file is a PE
    executable with the complete Flutter bundle embedded at its root.
  - `spull.exe` launch smoke stayed alive for five seconds with the complete
    runtime.
  - Release workflow YAML parsed successfully and `git diff --check` passed.
- Blockers: Linux and macOS native builds require their respective CI runners;
  this Windows host cannot execute those platform builds.
