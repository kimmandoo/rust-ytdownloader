# Work checkpoint

- Active task: Fixed Windows uninstaller launches from the setup folder.
- Next action: Commit the uninstaller fix, then publish `v1.2.3`.
- Changed files: `tool/Uninstall-Spull.ps1`, `tool/package_windows_release.ps1`,
  `README.md`, `CHANGELOG.md`, and this checkpoint.
- Runtime behavior: The uninstaller uses its own folder when it contains
  `spull.exe`; otherwise it resolves the default `%LOCALAPPDATA%\Spull`
  installation. Custom installations must use the copy installed in that
  folder.
- Verification:
  - PowerShell parser accepted the corrected uninstaller.
  - Direct uninstaller smoke removed a temporary test installation.
  - Windows setup packaging passed; the setup ZIP contained both uninstaller
    launch files and bundled `bin/ffmpeg.exe`.
  - `git diff --check` passed.
- Blockers: None.
