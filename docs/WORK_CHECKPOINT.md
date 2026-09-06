# Work checkpoint

- Active task: Added and verified stalled-download recovery.
- Next action: Commit the watchdog changes, then run
  `tool/publish_release.ps1 -Version 1.2.4` to create and push the release.
- Changed files: `lib/services/spull_backend.dart`, `lib/state/app_controller.dart`,
  `README.md`, `CHANGELOG.md`, and this checkpoint.
- Runtime behavior: Each media download arms a five-minute no-output watchdog.
  Any non-empty yt-dlp or FFmpeg output resets it. A stalled process emits an
  `응답 없음` event, terminates its process tree, retries once using yt-dlp
  resume support, and reports a visible failure if the retry stalls again.
  Manual cancellation remains a stop and never retries.
- Verification:
  - Temporary stalled-download smoke passed with two watchdog events, one
    automatic retry, and a final failure; the temporary script removed itself.
  - `dart run tool/verify.dart` passed formatting, analysis, and all widget tests.
- Release state: Current base is `f67e24d`; release target is `1.2.4`.
- Blockers: None.
