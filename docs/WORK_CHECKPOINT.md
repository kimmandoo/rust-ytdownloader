# Work checkpoint

- Active task: Completed v1.2.5 release publication and CI verification.
- Next action: No follow-up action; wait for the next requested task.
- Changed files: `lib/models/media_models.dart`, `lib/state/app_controller.dart`,
  `lib/services/spull_backend.dart`, `lib/home_page.dart`,
  `lib/widgets/pixel_widgets.dart`, `test/widget_test.dart`, `README.md`,
  `CHANGELOG.md`, `pubspec.yaml`, and this checkpoint.
- Runtime behavior: MP3/M4A expose persisted 128K/192K/256K/320K choices;
  MP4/WEBM expose persisted resolution caps from Best through 360p. WAV/FLAC
  remain lossless. yt-dlp receives bounded selectors and downloads launch
  without a shell so Windows format-filter characters remain intact. The
  download progress panel now appears before the potentially long queue and
  keeps a visible track at zero progress; long status messages wrap to two
  lines.
- Release state: `release-v1.2.5` points to `9a63264` and was pushed with
  `main` to `origin`; GitHub Actions run #53 completed successfully.
- Verification:
  - `dart run tool/verify.dart` passed formatting, analysis, and all widget
    tests after the status-bar repair and no-shell launch adjustment.
  - Temporary compiled-yt-dlp smoke passed bounded `1080p` video selectors and
    `192K` MP3 arguments after the no-shell launch adjustment; the temporary
    script removed itself.
- Blockers: None.
