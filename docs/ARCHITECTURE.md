# Architecture

## Overview

```
React / TypeScript (Vite)
        │
     Tauri IPC (typed commands + events)
        │
Rust application core (src-tauri)
 ├── model/        domain types (task, schedule, action, target, history, settings)
 ├── scheduler/    timer + reconciliation engine (authoritative)
 ├── executor/     ordered action-sequence engine
 ├── platform/     PlatformAutomation trait abstraction
 │    └── windows/ Win32 backend (enumeration, target, focus, input)
 ├── store/        schema-versioned JSON persistence (atomic writes)
 ├── service/      history + settings services
 └── commands.rs   Tauri command surface
```

## Core rules

1. **Scheduling lives in Rust.** The scheduler owns `next_run_at`, fires tasks, and emits
   Tauri events. Frontend timers/countdowns are display-only.
2. **Platform code is abstracted behind `PlatformAutomation`.** Core logic (scheduler,
   executor, matching) is platform-agnostic and unit-tested against a mock. Win32 FFI is
   confined to `platform/windows/`.
3. **Safety-first execution.** Every keyboard injection is preceded by foreground
   verification of the resolved target. Ambiguous resolution aborts. Focus loss aborts.
   Execution is globally serialized (one sequence at a time).
4. **Persistence is schema-versioned, atomic, and self-healing.** Writes go to a temp file
   then rename; a corrupted store falls back to the previous valid copy.

## Key data flow: task execution

```
Scheduler tick/reconcile
  → task due? apply misfire policy
  → acquire global executor lock (else skip; ExecutionAlreadyRunning surfaced)
  → resolve WindowTarget (cached HWND → verify; else enumerate+match; ≥2 matches ⇒ abort)
  → restore window
  → activate window
  → GetForegroundWindow() == target?  else abort FailedToActivateTarget
  → run actions in order (re-verify focus after delays ≥ ~3 s)
  → structured result → history record → Tauri event (task-completed / task-failed)
```

## Window target resolution

Persisted identity is rich (process name/path/PID, title + match mode, last HWND).
At runtime: verify cached HWND → if invalid, enumerate visible top-level windows → match
process, then title (Exact / Contains / Regex / Any) → require exactly one candidate.
Multiple matches ⇒ `TargetAmbiguous`; zero ⇒ `TargetNotFound`. Never guess.

## Threading model

- Tauri runs a Tokio-free async runtime for IPC; the scheduler uses a dedicated thread
  with a condvar wait (woken on task changes) — no busy loop, no polling.
- The executor runs on its own thread and holds the global input lock for the whole
  sequence.

## Persistence

Single JSON document `{ version, tasks, settings, history }` under the Tauri app-data
directory. See `docs/DECISIONS.md` for why JSON (not SQLite) and `docs/DEVELOPMENT.md`
for file semantics.
