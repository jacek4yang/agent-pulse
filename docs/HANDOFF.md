# Current Engineering Handoff

> Rolling checkpoint. Update after every milestone, before session end, after opening
> or merging PRs. Git holds history; this file holds CURRENT state only.

## Current milestone

v0.1.1 (post-MVP fixes released)

## Active Issue

None (v0.1.1 released; next work = v0.2.0 per ROADMAP)

## Active branch

main

## Active PR

None (release-prep PR merged; tag `v0.1.0` triggers `release.yml`)

## Current commit

See `git log --oneline -n 3`

## Completed (v0.1.0, all via squash-merged PRs)

- #5/#6 Foundation (PR #20) · #7 Persistence (PR #23) · #8 Scheduler (PR #22)
- #9/#10/#11 Windows platform (PR #24) · #12/#13 Executor + SendInput (PR #25)
- #14–#17 IPC + frontend (PR #26) · #18 Tray/notifications/autostart (PR #27)
- Dependabot action bumps (PRs #1, #2, #4; pnpm/action-setup v6 applied in release PR)
- 77 Rust unit tests; clippy `-D warnings` clean; fmt clean; pnpm lint/build clean;
  full Tauri Windows build green in CI on every PR

## Release procedure (executed)

1. Release-prep PR (`feat/19-release`): pnpm/action-setup v6, ROADMAP check-off,
   this file. Merged to main.
2. `git tag v0.1.0 && git push origin v0.1.0` from the merged main commit.
3. `release.yml` runs: Rust tests → `pnpm tauri build` → collects NSIS + MSI into
   `release-artifacts/` → writes `SHA256SUMS.txt` → publishes GitHub Release with
   generated notes.
4. Verify the release page lists real artifacts only (never fake filenames).

## Verification

Last successful local commands:

- `cargo fmt --all -- --check` ✓
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` ✓
- `cargo test --workspace` ✓ 77 passed
- `pnpm lint` ✓ / `pnpm build` ✓

## v0.1.1 addenda

- #29 (P0): commands requested `State<AppState>` while `Arc<AppState>` was managed —
  every stateful command failed. All signatures now use `State<Arc<AppState>>`.
- #30: Quick Automation supports After/At/Every with second precision and fully
  custom action flows; `create_quick_task` takes any schedule + actions.
- #32: full Chinese + English UI; language setting persisted in Settings.

## Known problems / deferred

- Manual interactive E2E suite (docs/TESTING.md: happy-path typing into a real
  terminal, ambiguity negative test, restart/sleep recovery, tray persistence)
  requires a human at the desktop; unit + CI coverage substitutes until run.
  Tracked in issue #19 before tagging.
- Release binaries are unsigned (SmartScreen may warn) — documented in
  docs/SECURITY.md; signing is a v1.0.0 item.
- Frontend `Notify` action currently emits `execution-notify` events; native
  toast for in-sequence notifications lands with v0.2.0 polish.

## Next exact actions (for the next agent)

1. Confirm `release.yml` succeeded for tag `v0.1.0` and artifacts + checksums exist.
2. If it failed: inspect `gh run list --workflow release.yml`, fix, re-tag
   (delete failed draft release first, then `git push origin :refs/tags/v0.1.0`
   and re-tag).
3. Begin v0.2.0 per ROADMAP.md (first issue: sleep/resume recovery telemetry).
4. Keep following the AGENTS.md loop: Issue → branch → PR → CI → squash merge.
