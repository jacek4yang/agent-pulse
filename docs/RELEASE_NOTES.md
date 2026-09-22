# v0.2.0 — Reliable Enter execution and desktop platform support

## Fixes

- Pausing, editing or deleting a task cancels remaining input, including during delays.
- Default Continue submits once; extra confirmation Enter is an explicit preset.
- Physical Windows Enter scan code; correct extended navigation key flags.
- Foreground checks on every repeated press and throughout text input. Disabling
  abort now requires successful reactivation and verification. Held modifier keys
  produce an error instead of accidental modified shortcuts.
- Prevent cached handles from bypassing ambiguity and restore process identity lookup.
- Reject text control characters, invalid action bounds, overflowing intervals and
  past absolute deadlines. Editing a completed task no longer unexpectedly re-arms it.
- Scheduled and manual runs share global serialization; duplicate app instances are
  prevented. Run Now executes off the UI thread.
- Recover overdue tasks on startup, process Skip-only schedules without a busy loop,
  preserve the original due time, and keep execution completion from overwriting edits.
- Persist settings/history and memory together, retaining old memory if saving fails.
- Wire tray, notifications and autostart; correct the tray state type. Show startup
  data errors in the application rather than panicking or silently displaying defaults.
- Preserve exact-time seconds/timezone and recurring seconds when editing.

## Packages

- Windows 10/11 x64: NSIS EXE and MSI.
- macOS 11+: universal DMG (Intel and Apple Silicon).
- Linux x64: DEB and AppImage, built on Ubuntu 22.04.
- SHA256SUMS.txt covers all installers.

## Requirements and limits

macOS requires Accessibility and Automation > System Events permission and a target
application with exactly one accessible window. Linux requires X11/Xorg, an EWMH
window manager and xdotool; Wayland is explicitly unsupported. DEB installs xdotool;
AppImage users must install it separately. Windows requires WebView2 and a target at
an equal or lower privilege level. Locked desktops and detached remote sessions are
not supported.

Existing task actions are preserved, including any explicit second Enter. Review
those actions before enabling them. Target the correct terminal tab/pane and leave
it selected. Success records acknowledge injected input, not application-level
command completion. macOS/Linux are newly added backends; no claim is made that
every OS version, terminal or desktop environment has been manually tested.

Windows/macOS binaries are unsigned; the macOS app is not notarized.

See docs/COMPATIBILITY.md and docs/TESTING.md for the verification scope.
