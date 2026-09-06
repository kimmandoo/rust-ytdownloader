# Work checkpoint

- Active task: Completed persisted audio/video quality controls and repaired
  download status visibility.
- Next action: No follow-up action; publish only if the user requests a new
  release.
- Changed files: `lib/models/media_models.dart`, `lib/state/app_controller.dart`,
  `lib/services/spull_backend.dart`, `lib/home_page.dart`,
  `lib/widgets/pixel_widgets.dart`, `test/widget_test.dart`, `README.md`,
  `CHANGELOG.md`, and this checkpoint.
- Runtime behavior: MP3/M4A expose persisted 128K/192K/256K/320K choices;
  MP4/WEBM expose persisted resolution caps from Best through 360p. WAV/FLAC
  remain lossless. yt-dlp receives bounded selectors and downloads launch
  without a shell so Windows format-filter characters remain intact. The
  download progress panel now appears before the potentially long queue and
  keeps a visible track at zero progress; long status messages wrap to two
  lines.
- Release state: `release-v1.2.4` remains the latest release tag and points to
  `898c441`; this feature was committed locally as `9c27d9d` and has not been
  released or pushed.
- Verification:
  - `dart run tool/verify.dart` passed formatting, analysis, and all widget
    tests after the status-bar repair and no-shell launch adjustment.
  - Temporary compiled-yt-dlp smoke passed bounded `1080p` video selectors and
    `192K` MP3 arguments after the no-shell launch adjustment; the temporary
    script removed itself.
- Blockers: None.
