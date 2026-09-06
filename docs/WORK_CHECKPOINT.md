# Work checkpoint

- Active task: Added a Windows uninstaller to the script-based setup bundle.
- Next action: Push `main`; publish a new release tag when the uninstaller is
  ready for users.
- Changed files: `tool/Install-Spull.ps1`, `tool/Uninstall-Spull.ps1`,
  `tool/Uninstall-Spull.vbs`, `tool/package_windows_release.ps1`, `README.md`,
  `CHANGELOG.md`, and this checkpoint.
- Runtime behavior: Installation copies both uninstaller files into the
  installed folder and creates an `Uninstall Spull` Start Menu shortcut when
  shortcuts are enabled. The uninstaller validates the install executable,
  asks for confirmation, stops the installed Spull process, removes only
  matching Start Menu shortcuts and the install directory, and reports errors.
- Verification:
  - PowerShell parser accepted the installer, uninstaller, and packaging
    scripts.
  - Local Windows packaging passed and produced portable plus setup ZIPs.
  - Setup ZIP contains `Uninstall-Spull.ps1` and `Uninstall-Spull.vbs`.
  - Nested runtime contains bundled `bin/ffmpeg.exe` (102,856,192 bytes).
  - `git diff --check` passed.
- Blockers: None.
