# Work checkpoint

- Active task: Corrected the PowerShell 7 Windows setup compilation failure
  and Linux artifact verification, then triggered the corrected release run.
- Next action: Inspect the completed run `33983711842` and published assets.
- Changed files: `.github/workflows/release.yml`,
  `tool/package_windows_release.ps1`, `tool/verify_desktop_artifact.dart`,
  changelog, and this checkpoint.
- CI result: Windows setup packaging now invokes the .NET Framework C# compiler
  directly, avoiding unsupported PowerShell 7 `Add-Type -OutputType` values.
  Linux verification now checks Flutter's `data/flutter_assets` layout instead
  of requiring Windows-only `data/app.so` and `icudtl.dat`.
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
  - The updated Windows package script compiled the setup launcher with the
    .NET Framework C# compiler and created the portable ZIP and setup EXE.
  - Both generated payloads were inspected; the setup file is a PE executable
    with the complete Flutter bundle embedded at its root.
  - `dart run tool/verify.dart` passed again: formatting, analyzer, and widget
    tests passed.
  - `dart run tool/verify_desktop_artifact.dart windows` passed.
  - The previous remote Linux build produced a valid bundle but the verifier
    incorrectly required Windows-only `data/app.so`; the verifier now matches
    Linux's `data/flutter_assets` layout.
  - Release workflow YAML parsed successfully and `git diff --check` passed.
  - GitHub Actions run `33983711842` is queued/in progress for tag
    `release-v1.2.0`; Linux and macOS native builds remain delegated to Actions.
