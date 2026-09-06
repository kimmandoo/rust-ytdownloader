# Work checkpoint

- Active task: Published the stalled-download watchdog release.
- Next action: Inspect the `release-v1.2.4` GitHub Actions run.
- Changed files: `lib/services/spull_backend.dart`, `lib/state/app_controller.dart`,
  `README.md`, `CHANGELOG.md`, and this checkpoint.
- Runtime behavior: Each media download arms a five-minute no-output watchdog.
  Any non-empty yt-dlp or FFmpeg output resets it. A stalled process emits an
  `응답 없음` event, terminates its process tree, retries once using yt-dlp
  resume support, and reports a visible failure if the retry stalls again.
  Manual cancellation remains an explicit stop and never retries.
- Release state: `release-v1.2.4` points to `898c441`; the release commit and
  tag were pushed to `origin` with the targeted branch-and-tag command.
- Verification:
  - Temporary stalled-download smoke passed with two watchdog events, one
    automatic retry, and a final failure; the temporary script removed itself.
  - `dart run tool/verify.dart` passed formatting, analysis, and all widget tests
    during release publishing.
- Blockers: None.
