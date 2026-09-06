# Work checkpoint

- Active task: Documented the safe targeted push for release tags.
- Next action: Inspect the `release-v1.2.1` GitHub Actions run; the tag and
  `main` branch are already pushed.
- Changed files: `README.md`, `CHANGELOG.md`, `docs/WORK_CHECKPOINT.md`.
- Release state: `release-v1.2.1` points to `cc99e2b`, and `origin/main` also
  points to `cc99e2b`.
- Push correction: `git push origin main release-v1.2.1` pushes only the
  requested branch and release tag. `git push origin main --tags` retries every
  local tag and is not documented or recommended.
- Verification:
  - `git diff --check` passed.
- Blockers: None. The old-tag rejection was harmless; the requested branch and
  release tag were accepted by the remote.
