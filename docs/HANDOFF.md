# Current Engineering Handoff

> Rolling checkpoint. Update after every milestone, before session end, after opening
> or merging PRs. Git holds history; this file holds CURRENT state only.

## Current milestone

v0.1.0 — MVP

## Active Issue

#19 Release pipeline: Windows bundles, checksums, v0.1.0 (last open v0.1.0 issue)

## Active branch

feat/12-tray (PR #27, stacked on PR #26)

## Active PR

- #26 feat(ui): IPC surface + dashboard, editor, picker, history, settings (CI running)
- #27 feat(desktop): tray, notifications, autostart, hide-to-tray (stacked on #26)

## Current commit

See `git log --oneline -n 3` (feat/12-tray)

## Completed (all merged to main)

- #5/#6 Foundation: Tauri 2.11 + React 19/TS strict, domain models, structured errors (PR #20)
- #7 Persistence: schema-versioned JSON store, atomic writes, corruption recovery (PR #23)
- #8 Scheduler: reconciliation engine, misfire policies, drift-free recurrence (PR #22)
- #9/#10/#11 Windows: enumeration, matching (ambiguity abort), safe focus (PR #24)
- #12/#13 Executor: action engine, SendInput (Unicode, no clipboard), global
  serialization, focus-loss abort (PR #25)
- #14–#17 IPC + full frontend (PR #26, merge pending CI)
- #18 Tray/notifications/autostart (PR #27, stacked)

Quality state: 77 Rust unit tests green, clippy -D warnings clean, fmt clean,
pnpm lint/build clean. CI runs all of these + full Tauri Windows build per PR.

## In progress

- Waiting on CI for PR #26, then merge #26 → merge #27
- 4 dependabot action-bump PRs (#1–#4) mergeable; merge AFTER #26/#27 to avoid churn

## Remaining acceptance criteria for v0.1.0

Issue #19 (release pipeline):
- [ ] Merge dependabot action bumps
- [ ] Verify release.yml: `v*` tag → tests → tauri build (NSIS + MSI) → SHA256SUMS.txt → GitHub Release
- [ ] Version sync (already 0.1.0 across tauri.conf.json / Cargo.toml / package.json)
- [ ] Manual E2E tests from docs/TESTING.md (happy path, ambiguity negative, restart, tray)
- [ ] Tag v0.1.0, verify Release artifacts exist and notes are accurate

## Verification

Last successful local commands:

- `cargo fmt --all -- --check` ✓
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` ✓ (0 errors)
- `cargo test --workspace` ✓ 77 passed
- `pnpm lint` ✓ / `pnpm build` ✓

## Known problems

- Rust CI jobs must run with `working-directory: src-tauri` (fixed in workflow)
- Stacked branches need main merged in before squash-merging (lib.rs add/add
  conflicts) — resolve by taking main's lib.rs + adding the branch's modules
- Release binaries are unsigned (documented in docs/SECURITY.md)

## Next exact actions

1. `gh pr checks 26` → merge #26 (`gh pr merge 26 --squash`)
2. Merge #27 (merge main into feat/12-tray first if conflicted)
3. Merge dependabot PRs #1–#4
4. Create branch `feat/19-release`: finalize docs (ROADMAP checkboxes, HANDOFF),
   verify release workflow; merge
5. Manual E2E tests (docs/TESTING.md) — requires interactive Windows session
6. `git tag v0.1.0 && git push origin v0.1.0` → release workflow builds & publishes
7. Verify GitHub Release artifacts (NSIS exe, MSI, SHA256SUMS.txt)
