# Current Engineering Handoff

## Current milestone
v0.2.0 reliability audit and Windows/macOS/Linux release, requested 2026-09-22.

## Active work
- Issue #45; branch `fix/45-enter-reliability`; PR #46 (validated; preparing squash merge).
- Base main: 5afa1f2 / v0.1.2. Dependency PRs #40-44 are independent.
- User specifically reported intermittent exits and unintended Enter while setting up.
- User expanded platform scope to macOS/Linux.

## Implemented
- Pausing/editing/deleting cancels pending input, including during delays.
- Windows partial input writes clean up this batch's held keys only while target remains foreground.
- Single default Enter; physical Windows Enter/extended keys; modifier rejection;
  repeated-key and text foreground checks; verified reacquisition; cached ambiguity fix.
- Restored process lookup, matched executable path, rejected empty target resolution.
- Shared execution guard and single app instance; async Run Now/window queries.
- Startup recovery, original due timestamps, Skip-only loop, bounded reconcile,
  checked schedule arithmetic, past-time rejection, no re-arm on ordinary edits.
- Transactional memory/disk settings/history and save rollback; completion preserves
  newer edits. Startup data errors stay visible without creating a writable empty store.
- Wired tray/plugins, corrected Arc state, close-to-tray and minimized startup.
- Editor preserves exact seconds/timezone; task failures surface on Run Now.
- macOS System Events and Linux X11 backends. Native bundle/release CI matrix.
- README, compatibility, release notes and regression docs updated.

## Verification checkpoint
- Windows local: 101 Rust tests passed, clippy -D warnings passed, fmt check passed.
- pnpm lint and build passed.
- Local Windows NSIS/MSI build succeeded. Final source native CI also passed.
- Windows Tk and Linux Xvfb/xterm Enter probes passed: exact text, one submission, no extra Enter.
- Final source a0c9dce: CI run 35696978747 passed all jobs. Windows 101 tests; macOS/Linux 92 each. All five installer types built. This checkpoint-only commit reruns CI before merge.

## Known limits
- No claim of universal terminal/OS manual testing. macOS needs Accessibility and
  Automation permission and exactly one accessible target window. Linux needs X11,
  an EWMH WM and xdotool; Wayland explicitly rejected.
- Window targeting cannot identify terminal tabs/panes. Success means input accepted.
- Unsigned Windows/macOS packages, no macOS notarization.
- Existing explicit second Enter actions are preserved; review old tasks.

## Next actions
Local gates and native CI are complete. Wait for checkpoint commit CI, then squash merge PR #46 only when green. Wait for green main CI, tag v0.2.0 (Release reuses that exact commit's CI artifacts), and verify five installer types
plus actual SHA256SUMS after all release builds. Update this checkpoint before merge
and after release. Do not move a published tag.

## Latest CI finding
Checkpoint run 35723228506 failed Linux native input with TargetNotFound while the
xterm window existed. Linux enumeration required readable /proc/PID/exe, which is
not guaranteed for setgid terminals. Preserve candidates using PID/title and optional
/proc/PID/comm when image paths are inaccessible; explicit path matching still fails
closed. Smoke probe now builds before opening its receiver and logs WM/client state.
Fix CI must pass before merge. No keyboard input was sent by the failed probe.
