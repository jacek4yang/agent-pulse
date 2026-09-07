# ROADMAP

## v0.1.0 — MVP

- [x] Repository foundation (docs, CI, governance)
- [x] Tauri 2 + React/TS application skeleton
- [x] Core domain models (task, schedule, action, target, history, settings)
- [x] Structured error model
- [x] Schema-versioned JSON persistence (atomic writes, corruption recovery)
- [x] Scheduler engine (relative / absolute / recurring, misfire policies, reconciliation)
- [x] Win32 window enumeration
- [x] Target resolution & matching (exact / contains / regex / any; ambiguity abort)
- [x] Safe foreground activation & verification
- [x] SendInput backend (Unicode typing, key combos)
- [x] Action sequence engine (ordered, focus-loss protection, global serialization)
- [x] Dashboard + quick creation mode
- [x] Automation editor (schedule + ordered action editing)
- [x] Window picker
- [x] Execution history UI
- [x] Settings (theme, notifications, startup options)
- [x] System tray (hide to tray, scheduler keeps running)
- [x] Notifications (success / failure)
- [x] Windows autostart option
- [x] CI (fmt, clippy, tests, frontend lint/build, full Tauri Windows build)
- [x] Release pipeline (MSI + NSIS + checksums) and v0.1.0 GitHub Release

## v0.2.0 — Reliability

- Improved sleep/resume recovery telemetry
- Target re-resolution diagnostics (why a target did/didn't match)
- Configurable misfire behavior per task (beyond RunImmediately / Skip)
- Richer execution diagnostics per action step
- Startup reliability hardening
- Settings migration tooling
- Accessibility pass (keyboard navigation, screen reader labels)
- Expanded integration testing

## v0.3.0 — Advanced Automation

- Reusable automation presets (user-defined, shareable)
- Additional action types (e.g. mouse click, scroll, window move/resize)
- Task templates
- Conditional execution where reliably detectable
- Possible ConPTY integration: launch and own the agent terminal process and read PTY
  output directly (enables semantic "agent finished / awaiting input" detection)

## v1.0.0 — Stable

- Hardened persistence and mature schema migration strategy
- Stable, documented config schema with backward compatibility guarantees
- Stable UX surface
- Release code-signing strategy documentation (and signing if certificates available)
- Production-grade diagnostics / log export
