# Decisions (ADR-style log)

Durable architectural decisions. Future agents must NOT re-litigate these without a
compelling, documented reason.

## D1 — Rust owns the scheduler; frontend timers are display-only

JS timers throttle, die with the webview, and cannot survive restarts. All schedule
state (`next_run_at`), firing, and misfire reconciliation happen in Rust. Status: **settled**.

## D2 — Tauri 2 + React + TypeScript + Vite + pnpm

Native-quality small Windows utility without Electron overhead; Rust core is mandatory
anyway (SendInput, EnumWindows). Strict TypeScript. pnpm as package manager. Status: **settled**.

## D3 — SendInput (Unicode) instead of clipboard

Typing never overwrites the user's clipboard. Unicode text is injected as
`KEYEVENTF_UNICODE` scan events, which works across terminal apps regardless of layout.
Status: **settled**.

## D4 — Rich window identity, not just HWND

HWNDs are recycled/invalidated. Persist process id/path/name + title + match mode +
last HWND; verify cached HWND first, re-resolve on invalidation. Status: **settled**.

## D5 — Ambiguous target ⇒ abort

If multiple windows match the target criteria, execution aborts with `TargetAmbiguous`
before any input. Guessing could type into the wrong window. Safety over convenience.
Status: **settled**.

## D6 — Foreground verification before every injection; abort on focus loss

After activation, `GetForegroundWindow()` must be the target before any key is sent.
Focus is re-checked before meaningful input steps and after long delays. Enabled by
default; the user may explicitly allow reacquisition. Status: **settled**.

## D7 — Global input serialization

Keyboard injection is system-global. Exactly one task may execute a sequence at any
time; other due tasks wait or surface `ExecutionAlreadyRunning`. Status: **settled**.

## D8 — No OCR / screen reading in MVP

Terminal UIs differ wildly (Windows Terminal, PowerShell, cmd, TUIs). Screen scraping is
fragile; deterministic sequences are safe. Future: ConPTY-managed agents where Agent
Pulse owns the process and reads PTY output. Status: **settled** for v0.1.0.

## D9 — JSON persistence with schema version, not SQLite

Dataset is tiny (≤ a few thousand records incl. bounded history). JSON with
`version` field, atomic temp-file+rename writes, corruption fallback, and a migration
path per version bump is simpler and auditable. Status: **settled** for v0.1.0.

## D10 — Misfire policy on reconciliation

On startup or scheduler wake, overdue tasks apply `MisfirePolicy::RunImmediately`
(default) or `Skip`; recurring tasks recompute the next occurrence from schedule
semantics (no drift accumulation). Status: **settled**.

## D11 — Tray-close ≠ quit

Closing the window hides to tray; the scheduler keeps running. Explicit Quit from the
tray menu terminates cleanly. Status: **settled**.

## D12 — No administrator privileges

The app uses documented, unprivileged APIs only (EnumWindows, SetForegroundWindow,
SendInput, ShowWindow). Status: **settled**.
