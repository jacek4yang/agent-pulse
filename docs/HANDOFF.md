# Current Engineering Handoff

## Current milestone
v0.2.0 reliability audit and Windows/macOS/Linux release, requested 2026-09-22.

## Active work
- Issue #45; branch `fix/45-enter-reliability`; PR #46 (draft).
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
- Local Windows NSIS/MSI build succeeded. Final source changes will be rebuilt by CI.
- Added isolated Linux Xvfb/xterm and Windows Tk Enter probes; Linux initial smoke passed; Windows and final source CI pending.
- Final cancellation/partial-input cleanup: local fmt, clippy and 101 tests passed.

## Known limits
- No claim of universal terminal/OS manual testing. macOS needs Accessibility and
  Automation permission and exactly one accessible target window. Linux needs X11,
  an EWMH WM and xdotool; Wayland explicitly rejected.
- Window targeting cannot identify terminal tabs/panes. Success means input accepted.
- Unsigned Windows/macOS packages, no macOS notarization.
- Existing explicit second Enter actions are preserved; review old tasks.

## Next actions
Finish local gates, open PR, inspect all native CI including Linux real Enter smoke,
fix failures, squash merge only green CI. Wait for green main CI, tag v0.2.0 (Release reuses that exact commit's CI artifacts), and verify five installer types
plus actual SHA256SUMS after all release builds. Update this checkpoint before merge
and after release. Do not move a published tag.
