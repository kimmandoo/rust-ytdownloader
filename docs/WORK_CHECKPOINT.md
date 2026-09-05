# Work checkpoint

- Active task: Corrected the invalid pinned Flutter action SHA and triggered the
  republish workflow successfully.
- Next action: Inspect the completed GitHub Actions run and published assets.
- Changed files: `.github/workflows/release.yml`, changelog, and this
  checkpoint.
- CI result: The Flutter action now uses the resolvable `v2` commit
  `1a449444c387b1966244ae4d4f8c696479add0b2`; the workflow still publishes
  the portable Windows ZIP and setup executable plus the Linux and macOS
  artifacts.
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
  - Corrected workflow YAML parsed successfully, and the action SHA matched the
    remote `subosito/flutter-action` `v2` tag.
  - GitHub Actions run `33983192239` started from `00d10b3`; all three verify
    jobs reached `Set up Flutter`, confirming that the action pin resolves.
- Blockers: the release workflow is still in progress; Linux and macOS native
  builds require their respective CI runners.
