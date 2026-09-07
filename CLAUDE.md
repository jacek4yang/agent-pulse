# CLAUDE.md

Claude-specific guidance for working on **Agent Pulse** (see `AGENTS.md` for the
repository-wide agent guide and the mandatory Context Recovery Protocol).

## Commands

```bash
pnpm install              # frontend deps (pnpm only, no npm/yarn)
pnpm tauri dev            # run the desktop app in dev mode
pnpm tauri build          # Windows release build (MSI + NSIS)
cargo test --workspace    # all Rust tests (from repo root)
pnpm lint && pnpm build   # frontend gates
```

## Project layout

```
src/            React + TypeScript frontend (Vite)
src-tauri/      Rust backend (Tauri 2 app)
  src/          application core: model/, scheduler/, executor/, platform/, store/, service/
docs/           architecture, decisions, handoff, release, security, testing
```

## Critical invariants

- The **Rust scheduler is the single source of truth** for `next_run_at` and execution.
  React countdowns are display-only — never `setTimeout` anything that must actually run.
- All Win32 FFI lives in `src-tauri/src/platform/windows/`. Core business logic is
  platform-agnostic and tested against the `PlatformAutomation` mock — unit tests must
  never type into the developer's real desktop.
- Target ambiguity ⇒ `TargetAmbiguous` error, abort, zero input injected.
- Persistence is schema-versioned JSON with atomic writes under the Tauri app-data dir.

## Claude-specific notes

- On resume/compaction, follow the Context Recovery Protocol in `AGENTS.md` first.
  `docs/HANDOFF.md` is the authoritative checkpoint — trust it over chat history.
- Keep `docs/HANDOFF.md` updated before ending any work session.
- Use `gh` for all GitHub operations; the authenticated account owns the repo.
- Conventional commits; one branch per Issue; squash-merge PRs.
