# Work checkpoint

- Active task: Repaired Spull's desktop CI while completing the dynamic yt-dlp
  extractor catalog.
- Next action: The session's changes are committed; verify the branch state
  before delivery.
- Changed files: `.github/workflows/release.yml`, `tool/verify.dart`,
  `tool/verify_desktop_artifact.dart`, dynamic extractor model/backend/state/UI
  files, README, changelog, and this checkpoint.
- CI result: GitHub Actions now runs only for `release-*` tags or manual
  dispatch, validates `release(scope):` commit subjects, pins Flutter/action
  dependencies, verifies all three desktop bundles before packaging, and
  publishes complete archives only after all matrix builds pass.
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
  - Windows archive packaging smoke passed with runtime files at archive root.
  - `spull.exe` launch smoke stayed alive for five seconds with the complete
    runtime.
  - Release workflow YAML parsed successfully and `git diff --check` passed.
- Blockers: Linux and macOS native builds require their respective CI runners;
  this Windows host cannot execute those platform builds.
