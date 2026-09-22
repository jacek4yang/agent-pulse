# Agent Pulse

Schedule a keyboard sequence against a specific foreground application window.
The Rust scheduler supports delays, exact times with timezones, recurring tasks,
and persisted deadlines across restarts.

## v0.2.0

- **One Enter by default**: Continue types `continue` and submits once. Double Enter
  and Continue + Confirm remain explicit choices. Existing task actions are preserved.
- Foreground checks before each repeated key and during text entry; no input on
  ambiguous targets or failed activation. Held modifiers abort instead of changing
  Enter into Shift/Ctrl/Alt+Enter.
- One application instance and one executing sequence. Concurrent attempts report
  `ExecutionAlreadyRunning` in history instead of interleaving keyboard input.
- Fixed startup recovery, task edits re-arming completed tasks, schedule overflow,
  settings/history persistence, tray state and blocking Run Now commands.
- Windows, macOS and Linux X11 backends and installers. See the boundaries below.

## Downloads and compatibility

Get installers and `SHA256SUMS.txt` from [GitHub Releases](https://github.com/jacek4yang/agent-pulse/releases).

| Platform | Package | Requirements / scope |
| --- | --- | --- |
| Windows 10/11 x64 | NSIS `.exe` or `.msi` | Interactive unlocked desktop; WebView2; target at equal or lower privilege level |
| macOS 11+ Intel / Apple Silicon | Universal `.dmg` | Accessibility and Automation permission for System Events; exactly one accessible window in the target application |
| Linux x64 | `.deb` or `.AppImage` | X11/Xorg with an EWMH window manager and `xdotool`; built on Ubuntu 22.04 |

Wayland, Windows 7/8, locked/secure desktops and detached remote sessions are not
supported automation environments. macOS/Linux are new backends: build and unit
coverage does not establish compatibility with every terminal or desktop.
Windows and macOS downloads are unsigned/not notarized.

On Linux, install `xdotool` (the Debian package declares this dependency). AppImage
users also need their distribution's FUSE compatibility runtime. On macOS, allow
Agent Pulse to control System Events under System Settings > Privacy & Security >
Automation and Accessibility. Permission denial is an error, not a successful run.

## Use

1. Open the terminal and select the correct tab/pane. Agent Pulse targets a **window**;
   it cannot identify a terminal's active tab, pane, prompt or editor focus.
2. Select that target in Agent Pulse and set a future deadline.
3. Choose Continue (one Enter), or explicitly configure other actions.
4. Save. Use History for errors and actual execution timestamps.

The target must be available and unlocked at execution time. Avoid typing or changing
focus while a sequence runs. A successful history record means the input API accepted
the sequence; it cannot prove an AI agent resumed or a command completed.

Text actions reject control characters, including CR/LF: add explicit Enter actions.
Selecting a window or testing a target does not inject input. Saving an absolute time
in the past is rejected. Editing only a name/action preserves the existing deadline.
Changing a schedule intentionally re-arms it. Pause tasks before changing their setup
if an already scheduled deadline could occur while editing.

## Development

```bash
pnpm install --frozen-lockfile
pnpm tauri dev
pnpm lint
pnpm build
cd src-tauri
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

CI builds Windows NSIS/MSI, a universal macOS DMG, and Linux DEB/AppImage. Publication
waits for every platform and includes SHA-256 hashes for the actual installers.

See [testing](docs/TESTING.md), [compatibility](docs/COMPATIBILITY.md),
[architecture](docs/ARCHITECTURE.md), [handoff](docs/HANDOFF.md), and [roadmap](ROADMAP.md).

## License

MIT — see [LICENSE](LICENSE).
