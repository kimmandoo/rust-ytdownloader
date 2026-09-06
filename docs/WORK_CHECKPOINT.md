# Work checkpoint

- Active task: Automated the `v1.2.2` release commit, tag, and targeted push.
- Next action: Inspect the `release-v1.2.2` GitHub Actions run.
- Changed files: `tool/publish_release.ps1`, `README.md`, `CHANGELOG.md`, and
  this checkpoint.
- Release state: `release-v1.2.2` points to `e5d0cd1`; `origin/main` also
  contains that release commit.
- Publisher behavior: It requires a clean `main` worktree, rejects existing
  local or remote release tags, updates `pubspec.yaml`, runs verification,
  creates `release(vX.Y.Z): publish desktop artifacts`, creates an annotated
  `release-vX.Y.Z` tag, and pushes only `main` plus that tag.
- Verification:
  - Publisher execution for `v1.2.2` passed.
  - `dart run tool/verify.dart` passed: formatting, Flutter analyzer, and widget
    tests.
  - The remote accepted `main` and only `release-v1.2.2`; no old tags were
    pushed.
- Blockers: None.
