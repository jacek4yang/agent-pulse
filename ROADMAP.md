# ROADMAP

## v0.1.0 — MVP

- [x] Repository foundation (docs, CI, governance)
- [ ] Tauri 2 + React/TS application skeleton
- [ ] Core domain models (task, schedule, action, target, history, settings)
- [ ] Structured error model
- [ ] Schema-versioned JSON persistence (atomic writes, corruption recovery)
- [ ] Scheduler engine (relative / absolute / recurring, misfire policies, reconciliation)
- [ ] Win32 window enumeration
- [ ] Target resolution & matching (exact / contains / regex / any; ambiguity abort)
- [ ] Safe foreground activation & verification
- [ ] SendInput backend (Unicode typing, key combos)
- [ ] Action sequence engine (ordered, focus-loss protection, global serialization)
- [ ] Dashboard + quick creation mode
- [ ] Automation editor (schedule + ordered action editing)
- [ ] Window picker
- [ ] Execution history UI
- [ ] Settings (theme, notifications, startup options)
- [ ] System tray (hide to tray, scheduler keeps running)
- [ ] Notifications (success / failure)
- [ ] Windows autostart option
- [ ] CI (fmt, clippy, tests, frontend lint/build, full Tauri Windows build)
- [ ] Release pipeline (MSI + NSIS + checksums) and v0.1.0 GitHub Release

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
