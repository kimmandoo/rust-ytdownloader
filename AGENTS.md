# Rules

Commit per each query session.
Commit messages must follow `type(scope): subject`, for example `feat(trading): harden futures runtime`.
When strategy code, strategy defaults, or strategy selection behavior changes, run the relevant backtest before completion and report the result.

## CI

- Run GitHub Actions automatically only when a `release-*` tag points to a
  release commit whose subject starts with `release(scope):`, for example
  commit `release(v1.0.0): publish desktop artifacts` with tag
  `release-v1.0.0`. A release commit without the tag remains local to Actions.
- Ordinary feature, fix, documentation, and build commits must not use the
  `release(scope):` prefix. Keep `workflow_dispatch` available for a manual
  release verification or rerun.

# Changelog

When a feature is added, a bug is fixed, or any breaking change is introduced, upsert to the CHANGELOG.md file.
The changelog should be written in the past tense and follow the same format as the commit messages.
Group changelog entries under reverse-chronological `## YYYY-MM-DD` headings using the date of the change.
Add new entries under the current date heading, creating it when needed.

## Intermediate checkpoints and continuation

- Maintain `docs/WORK_CHECKPOINT.md` as the repository handoff record for work
  that may continue in a later query session.
- At the start of a resumed session, read `docs/WORK_CHECKPOINT.md`, this file,
  `TASKS.md`, and the active implementation-plan section before changing code.
- Confirm `git status --short --branch` and the latest commit. Continue the
  exact active task and recorded plan step; never infer that a task is complete
  from a commit title or jump to the next task because the working tree is
  clean.
- Before ending a session, update the checkpoint with the active task, exact
  next action, changed files, verification commands and results, and blockers.
- Commit the checkpoint together with the session's changes. If work is still
  incomplete, keep the task active and record the next RED/GREEN or diagnostic
  step. Mark completion only after the plan's required verification passes.
