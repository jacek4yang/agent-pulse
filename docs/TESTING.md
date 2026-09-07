# Testing guide

## Automated (CI — must stay green)

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
pnpm lint
pnpm build
```

Plus one CI job performing a real `tauri build` on `windows-latest`.

## Rust unit test coverage map

| Area | Required cases |
|---|---|
| Schedule math | relative delay, absolute time, recurring interval, next occurrence, overdue, drift-free recurring |
| Misfire | RunImmediately, Skip, restart recovery, sleep-like large time jump |
| Persistence | task round-trip, Action enum round-trip, settings round-trip, history round-trip, schema version handling, corruption handling |
| Window matching | exact / contains / regex title, process match, cached HWND invalidation, 0 / 1 / ≥2 matches (≥2 ⇒ ambiguous) |
| Executor | exact action ordering, activation failure ⇒ no input, focus loss ⇒ abort, two tasks ⇒ serialized, input failure ⇒ structured failure |

Executor tests use the mock `PlatformAutomation` and never touch the real desktop.

## Manual end-to-end (before release)

1. **Happy path**: schedule a 10 s task targeting Windows Terminal: TypeText `continue`,
   PressKey Enter, Delay 1000, PressKey Enter. Switch to another app. Verify: target
   found, restored, activated, foreground confirmed, text typed, both Enters sent,
   history success.
2. **Ambiguity (negative)**: create title criteria matching ≥2 windows, run, verify
   `TargetAmbiguous` in history and that **no** input was sent anywhere.
3. **Restart**: schedule a future task, quit properly, relaunch before due time, verify
   it still fires.
4. **Sleep/recovery**: schedule 5 min out, sleep the machine past due, resume, verify
   misfire policy applies.
5. **Tray**: schedule a task, close the window to tray, verify it still fires.
6. **Focus-loss abort**: start a long sequence and click into another app; verify abort
   before next input step.
