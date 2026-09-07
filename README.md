# Agent Pulse

A lightweight Windows automation scheduler for reliably resuming terminal-based AI coding agents.

Agent Pulse schedules **keyboard action sequences** that run against a **specific Windows window** at a chosen time. Its flagship use case: when a CLI AI agent (Codex CLI, Claude Code, …) pauses because a usage limit was exhausted, Agent Pulse can wait — e.g. 5 hours — then bring the terminal back to the foreground and type `continue` + Enter for you.

## Primary use case

```
After 5h 5m:
  1. Activate Windows Terminal (the window running your agent)
  2. Type "continue"
  3. Press Enter
  4. Wait 1000 ms
  5. Press Enter   (confirm any follow-up prompt)
```

## Features (v0.1.0)

- **Scheduler in Rust** — relative (`after 5h 5m`), absolute (`at …`), and recurring (`every 30 minutes`) schedules, persisted as real timestamps; survives restarts and reconciles missed runs after sleep/hibernate
- **Window targeting** — pick any visible top-level window (title, process name, PID, icon); matches by process and title (exact / contains / regex); richer identity is persisted, never just an HWND
- **Safety model** — input is injected only after the target is verified foreground; ambiguous targets abort; execution aborts if the target loses focus mid-sequence; one sequence at a time globally
- **Action sequences** — `Focus`, `Restore`, `TypeText`, `PressKey`, `KeyCombination`, `Delay`, `Notify`, executed strictly in order; fully user-editable
- **Presets** — Continue, Continue + Confirm (default), Confirm + Continue, Double Enter, Empty Submit
- **Unicode typing** — via `SendInput` keyboard events; never touches your clipboard
- **Persistence** — schema-versioned JSON with atomic writes, corruption detection and recovery
- **System tray** — hides to tray, scheduling continues; notifications for success/failure
- **History** — bounded execution history with structured errors and durations

## Safety model

Agent Pulse injects keyboard input only after verifying that the **intended target window is the foreground window**. If the target cannot be resolved uniquely, cannot be activated, or loses focus during a sequence, execution aborts before any further input is sent. Ambiguity never resolves to a guess.

## Limitations

- Agent Pulse v0.1.0 does **not** semantically inspect terminal output — it runs deterministic action sequences, not screen-reading logic. (ConPTY-based agent inspection is a roadmap item.)
- Windows only.
- Release binaries are not code-signed.

## Development

```bash
pnpm install
pnpm tauri dev

pnpm lint && pnpm build
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

See [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) and [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Roadmap

See [ROADMAP.md](ROADMAP.md).

## License

MIT — see [LICENSE](LICENSE).
