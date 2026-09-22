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

## v0.2.0 regression and release checks

Run Rust commands from `src-tauri` (the Rust manifest is there). CI runs all Rust
checks and builds bundles on Windows, macOS and Linux. Unit tests never inject
into the user's desktop. The native macOS/Linux backends need interactive acceptance
checks in addition to CI compile/test coverage (see COMPATIBILITY.md).

Regression coverage added: repeated Enter focus loss, cached ambiguity, newline
rejection, physical Enter/extended navigation events, process image lookup, schedule
overflow, decades of missed intervals, original scheduled timestamp, Skip-only live
loop, completed-task edit, exact schedule re-arm, memory/disk settings/history and
failed-save rollback, completion preserving newer edits.

Before relying on a terminal, use a disposable prompt to verify **one** expected Enter.
Repeat with a double-Enter preset only when explicitly selected. Check that selecting
windows, testing targets, editing names/actions and changing settings send zero input.
Also test application restart past a deadline, two instances, simultaneous tasks,
active modifiers, failure to activate and a focus switch between repeated key presses.
On macOS test denied/granted permissions and multiple windows. On Linux test an X11
session and the explicit Wayland rejection. Installer outputs are unsigned.

CI also runs native input probes in disposable Windows (Tk receiver) and Linux
(Xvfb + Openbox + xterm) sessions. They assert exact text and one submission, with
a two-second observation window for unintended extra Enter. These never run during
unit tests and are not invoked on a developer desktop.
