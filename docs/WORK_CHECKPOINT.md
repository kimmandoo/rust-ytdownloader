# Work checkpoint

- Active task: Replaced the unsigned custom Windows setup EXE with an
  antivirus-safe script-based GUI setup bundle and triggered the corrected run.
- Next action: Inspect completed run `33984730323` and the published assets.
- Changed files: `.github/workflows/release.yml`,
  `tool/package_windows_release.ps1`, `tool/Install-Spull.ps1`,
  `tool/Install-Spull.vbs`, README, changelog, and this checkpoint.
- CI result: Windows releases now publish the portable ZIP plus
  `spull-windows-x86_64-setup.zip`; users extract it and double-click the
  hidden PowerShell launcher to open the GUI installer without a custom EXE.
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
  - The setup package now contains `Install-Spull.vbs`,
    `Install-Spull.ps1`, and the complete runtime ZIP.
  - The VBS launcher starts Windows PowerShell hidden; the installer itself
    presents the folder, shortcut, progress, completion, and error dialogs.
  - Portable ZIP and setup ZIP packaging passed locally.
  - `dart run tool/verify.dart` passed: formatting, analyzer, and widget tests.
  - `dart run tool/verify_desktop_artifact.dart windows` passed.
  - Release workflow YAML parsed successfully and `git diff --check` passed.
- GitHub Actions run `33984730323` is queued for tag `release-v1.2.0`.
