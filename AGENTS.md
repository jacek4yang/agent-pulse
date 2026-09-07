# AGENTS.md — Agent repository guide for AI coding agents

`Agent Pulse` is a Windows desktop app (Rust + Tauri 2 + React/TypeScript) that schedules
keyboard action sequences against specific application windows. Its primary use case is
reliably resuming terminal AI agents (Codex CLI, Claude Code) when usage limits reset.

## Context Recovery Protocol (MANDATORY)

Whenever an agent starts or resumes work, BEFORE changing code, run:

```bash
pwd
git status --short --branch
git remote -v
git log --oneline --decorate -n 15
gh auth status
gh repo view
gh issue list --state open
gh pr list --state open
```

Then read, in order:

1. `AGENTS.md` (this file)
2. `CLAUDE.md`
3. `ROADMAP.md`
4. `docs/HANDOFF.md`  ← **the current engineering checkpoint; most important**
5. `docs/ARCHITECTURE.md`
6. `docs/DECISIONS.md`

Then determine: current milestone, active Issue, active branch, active PR, completed work,
remaining acceptance criteria, last tests run, known failures, and the next concrete action.

Do NOT begin new unrelated work until this reconstruction is complete. If the user message
is only `continue`, do not ask what was being done — recover from the above and continue
the highest-priority unfinished work.

## Non-negotiable rules

- **All substantive development after bootstrap goes through a branch + Pull Request.**
  Never commit feature work directly to `main`. Squash-merge via `gh pr merge --squash --delete-branch`.
- **Every feature/fix maps to a GitHub Issue.** Branch naming: `feat/12-scheduler`,
  `fix/31-resume-recovery`, `docs/42-release`, `chore/50-ci`.
- **Scheduling lives in Rust.** Never make the frontend responsible for execution timing;
  frontend timers are display-only.
- **Never inject keyboard input without verifying the resolved target is the foreground
  window.** Ambiguous target resolution aborts. No input ever goes to a "best guess" window.
- **Win32/unsafe code stays isolated** under `src-tauri/src/platform/windows/` with a
  safety comment on every `unsafe` block.
- **No `unwrap()`/`expect()`/`panic!`** in recoverable runtime paths — typed errors only
  (`src-tauri/src/error.rs`).
- **The repository must always be continuable from a cold context.** `docs/HANDOFF.md`
  is updated after every significant milestone, before ending a session, and before
  opening/merging PRs.

## Quality gates (all must pass before any PR merge)

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
pnpm lint
pnpm build
```

CI runs these on `windows-latest`. A PR with failing CI is not merged.

## Workflow per Issue

1. Read Issue; confirm acceptance criteria
2. `git switch -c <type>/<issue>-<slug>` from up-to-date `main`
3. Update `docs/HANDOFF.md` (active branch/Issue)
4. Implement; keep Win32 out of core logic
5. Run all quality gates locally
6. Commit (conventional commits, e.g. `feat(scheduler): persist absolute next-run timestamps`)
7. Push, `gh pr create` (Summary / Changes / Validation checklist / `Closes #XX`)
8. Watch `gh pr checks`; fix failures; merge squash; `git switch main && git pull --ff-only`
9. Confirm the Issue auto-closed; update `docs/HANDOFF.md` and ROADMAP if needed
