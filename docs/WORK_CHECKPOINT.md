# Work checkpoint

- Active task: Added cancellable yt-dlp analysis and resilient bootstrap progress.
- Next action: Commit the implementation and inspect the next release workflow run.
- Changed files: `lib/services/spull_backend.dart`, `lib/state/app_controller.dart`,
  `lib/home_page.dart`, `lib/widgets/pixel_widgets.dart`, `test/widget_test.dart`,
  `README.md`, `CHANGELOG.md`, and this checkpoint.
- Runtime behavior: Link analysis has a 90-second process deadline and a
  `CANCEL SCAN` control. Extractor catalog loading has a 30-second deadline and
  its own cancel control. Windows and POSIX process trees are terminated when
  cancellation or timeout occurs.
- Bootstrap behavior: Dependency downloads report transfer speed and ETA when
  the response exposes a total size. Missing `Content-Length` now uses an
  indeterminate progress bar and reports received bytes plus speed.
- Documentation: README was reorganized around features, setup, dependency
  resolution, release artifacts, publishing, and project layout.
- Verification:
  - `dart run tool/.tmp_process_smoke.dart` passed native analysis and
    supported-sites process cancellation; temporary smoke files were removed.
  - `flutter analyze` passed with no issues.
  - `dart run tool/verify.dart` passed formatting, analysis, and all widget tests.
- Blockers: None.
