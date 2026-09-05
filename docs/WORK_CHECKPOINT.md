# Work checkpoint

- Active task: Converted the Windows setup asset into a GUI installer wizard
  without a console window.
- Next action: Create the republish release commit, move `release-v1.2.0` to
  it, and push the corrected commit and tag.
- Changed files: `tool/package_windows_release.ps1`, changelog, and this
  checkpoint.
- CI result: The setup launcher now compiles as a WinForms `winexe`; it shows
  an installation-folder wizard, optional Start Menu shortcut and launch
  choices, progress state, completion dialog, and error dialogs.
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
  - The updated Windows package script compiled the setup launcher and created
    the portable ZIP and GUI setup EXE.
  - The setup EXE passed PE validation with Windows GUI subsystem `2`; no
    console subsystem is present.
  - The setup payload was inspected and contains the complete Flutter runtime.
  - `dart run tool/verify.dart` passed: formatting, analyzer, and widget tests.
  - `dart run tool/verify_desktop_artifact.dart windows` passed.
  - Release workflow YAML parsed successfully and `git diff --check` passed.
  - The release publisher now checks out source before
    `gh release create --verify-tag`; run `33984088001` is in progress.
- Blockers: the external release run must finish on Linux/macOS/Windows
  runners before the assets are published.
