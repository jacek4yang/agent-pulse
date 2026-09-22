# Release process

## Versioning

Semantic versioning. The canonical version lives in `src-tauri/tauri.conf.json` (`version`)
and is mirrored in `src-tauri/Cargo.toml` and `package.json`. Bump all three in the
release-prep PR (or via script).

## Steps to cut a release

1. All milestone Issues closed or explicitly deferred; ROADMAP reflects reality.
2. Release-prep PR: version bumps + `docs/HANDOFF.md` refresh + release notes draft.
3. CI green on the PR; merge (squash) to `main`.
4. Tag: `git tag v0.1.0 && git push origin v0.1.0` (from the merged `main` commit).
5. The `release.yml` workflow (trigger: `v*` tags) then:
   - runs tests on `windows-latest`
   - `tauri build` producing NSIS `-setup.exe` and MSI bundles
   - computes `SHA256SUMS.txt` over all artifacts
   - creates the GitHub Release with actual built filenames and notes
6. Verify the release page lists the real artifacts; do not fake filenames.

## Rules

- Never build/publish artifacts that were not actually produced by CI.
- No signing keys in the repo. Code signing is deferred (see docs/SECURITY.md).
- If the release workflow fails, fix and re-tag (`v0.1.0-rc2` style test tags are fine;
  delete failed drafts before re-running).

## v0.2.0 release procedure

Version must match package.json, src-tauri/tauri.conf.json, src-tauri/Cargo.toml and
src-tauri/Cargo.lock. Tag only the green squash-merged commit. release.yml performs
native quality gates/builds for Windows x64, universal macOS and Linux x64, uploads
job artifacts, and publishes only after all three succeed. The publishing job requires
EXE/MSI/DMG/DEB/AppImage, rejects duplicate filenames and writes SHA256SUMS.txt over
installers only. Notes come from docs/RELEASE_NOTES.md. Verify GitHub assets and hashes
before reporting completion. Do not move a published release tag to repair a build.
