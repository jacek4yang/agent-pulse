# Development guide

## Prerequisites

- Rust stable (≥ 1.90) with the MSVC toolchain
- Node.js ≥ 20 and **pnpm** (`corepack enable`)
- Tauri 2 prerequisites on Windows: WebView2 (preinstalled on Win 10/11) and VS Build Tools with C++ workload

## Commands

```bash
pnpm install               # frontend deps
pnpm tauri dev             # dev run (hot reload)
pnpm tauri build           # Windows release bundle (NSIS + MSI)
pnpm lint                  # eslint
pnpm build                 # tsc + vite production build

cargo fmt                  # format Rust
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

## Layout

```
src/                React frontend (pages/, components/, lib/, styles)
src-tauri/
  src/
    main.rs         entry
    lib.rs          Tauri app builder, state setup
    commands.rs     typed IPC command surface
    error.rs        structured error model (serializable to frontend)
    model/          domain types
    scheduler/      engine
    executor/       action engine
    platform/       PlatformAutomation trait + windows/ backend
    store/          JSON persistence
    service/        history/settings
  tauri.conf.json   Tauri config (bundle identifiers, version)
```

## Persistence semantics

Single file `store.json` (plus `store.json.bak` previous-good copy) in the Tauri
app-data dir:

1. Serialize `{ version, tasks, settings, history }`
2. Write to `store.json.tmp`, fsync, rename over `store.json` (atomic on NTFS)
3. On load: parse error ⇒ try `.bak`; both corrupt ⇒ start empty and surface a
   `PersistenceFailure` notification (never silently destroy user data)
4. `version` < current ⇒ run migration steps; `version` > current ⇒ refuse load
   (newer binary would have written it; surface error)

History is bounded (default max 1000 records; oldest evicted).

## Testing rules

- Core logic (schedule math, matching, persistence, executor ordering) is pure Rust and
  tested with `#[cfg(test)]` unit tests.
- `PlatformAutomation` has a mock implementation; executor tests use it. **Unit tests
  must never inject input into the real desktop.**
- Manual end-to-end checks (real Windows Terminal typing) are documented in
  `docs/TESTING.md` and only run interactively.

## Git workflow

One branch per Issue, conventional commits, squash-merge PRs, CI green before merge.
See `AGENTS.md` for the full loop and `docs/HANDOFF.md` for current state.
