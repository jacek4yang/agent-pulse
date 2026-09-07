# Security notes

## What Agent Pulse can do

Agent Pulse injects keyboard input into arbitrary focused windows on a schedule. In the
worst case, a maliciously configured sequence could type commands into a terminal that
happens to be the target. This is inherent to the product's purpose; the mitigations
below are therefore **process** mitigations, not sandboxing.

## Mitigations in v0.1.0

- **Foreground verification before injection** — `GetForegroundWindow()` must equal the
  resolved target immediately before any key is sent.
- **Ambiguity abort** — multiple matching windows ⇒ `TargetAmbiguous`, zero input sent.
- **Activation failure abort** — target could not be brought to foreground ⇒
  `FailedToActivateTarget`, zero input sent.
- **Focus-loss abort** (default) — if the target loses focus mid-sequence, the sequence
  aborts before the next input step.
- **Global serialization** — only one sequence can inject at a time; sequences never
  interleave.
- **No clipboard use** — typing uses `SendInput` Unicode events; the clipboard is never
  read or written.
- **No elevated privileges** — the app runs unelevated and uses documented,
  unprivileged APIs only.
- **No network access** — the app makes no network calls; all state is local.
- **Structured errors surfaced to the user** — failures are visible in history and via
  notifications, including when no input was sent.

## Not claimed

- We do not claim protection against a user deliberately configuring a dangerous target
  (e.g. an always-on-top window stealing focus races) or against third-party software
  that fights for focus.
- Release binaries are currently **unsigned**; Windows SmartScreen may warn on first
  run. Code signing is a documented v1.0.0 hardening item.

## Reporting

Please open a GitHub Issue for security-relevant defects. For sensitive reports, mark
the issue clearly; do not include reproducible exploit chains in public issues.
