# Work checkpoint

- Active task: Repaired and published the Windows uninstaller fix.
- Next action: Inspect the `release-v1.2.3` GitHub Actions run.
- Changed files: `tool/Uninstall-Spull.ps1`, `tool/package_windows_release.ps1`,
  `README.md`, `CHANGELOG.md`, and this checkpoint.
- Runtime behavior: The uninstaller uses its own folder when it contains
  `spull.exe`; otherwise it resolves the default `%LOCALAPPDATA%\Spull`
  installation. Custom installations must use the copy installed in that
  folder. Matching shortcuts are removed without touching other Spull installs.
- Release state: `release-v1.2.3` points to `e449336`; `origin/main` contains
  the release commit.
- Verification:
  - PowerShell parser accepted the corrected installer and uninstaller.
  - Direct uninstaller smoke removed a temporary test installation.
  - Windows setup packaging passed; the setup ZIP contained both uninstaller
    launch files and bundled `bin/ffmpeg.exe`.
  - `dart run tool/verify.dart` passed during publisher execution.
- Blockers: None.
