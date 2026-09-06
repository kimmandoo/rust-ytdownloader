# Work checkpoint

- Active task: Added a guarded PowerShell publisher for release commits and
  targeted tag pushes.
- Next action: Commit the publisher changes, then run it for `v1.2.2`.
- Changed files: `tool/publish_release.ps1`, `README.md`, `CHANGELOG.md`, and
  this checkpoint.
- Publisher behavior: It requires a clean `main` worktree, rejects existing
  local or remote release tags, updates `pubspec.yaml`, runs verification,
  creates `release(vX.Y.Z): publish desktop artifacts`, creates an annotated
  `release-vX.Y.Z` tag, and pushes only `main` plus that tag.
- Verification:
  - PowerShell parser accepted the publisher script.
  - `git diff --check` passed.
- Blockers: None.
