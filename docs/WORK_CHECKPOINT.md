# Work checkpoint

- Active task: Repaired the incomplete Windows release artifact; the dynamic yt-dlp extractor catalog remains the next implementation task.
- Next action: Implement dynamic extractor discovery and searchable rendering from `yt-dlp --list-extractors`.
- Changed files: Windows release packaging workflow, release documentation, changelog, and this checkpoint.
- CI result: The Windows matrix now archives the complete `build/windows/x64/runner/Release` directory as `spull-windows-x86_64.zip`; Linux remains a complete tarball and macOS remains a complete app archive.
- Verification:
  - Windows archive smoke contained `spull.exe`, `data/`, and `flutter_windows.dll` after extraction.
  - Release workflow YAML parsed successfully.
  - Intel macOS matrix removal remains intact.
  - `git diff --check` passed before commit.
- Blockers: The dynamic extractor catalog is not implemented yet; Linux and macOS native builds require their respective CI runners.
