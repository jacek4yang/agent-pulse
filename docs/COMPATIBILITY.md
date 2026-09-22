# Compatibility and verification scope

## Supported execution environments

- **Windows 10/11 x64:** native SendInput. Enter uses scan code `0x1c`; text uses Unicode
  events. Navigation keys carry the extended flag. Release Shift/Ctrl/Alt/Windows keys.
  Use an unlocked interactive session and a target at equal/lower integrity level.
- **macOS 11+ Intel/Apple Silicon:** System Events via bounded osascript calls. Grant
  Accessibility and Automation permissions. A target app must expose exactly one
  accessible window; multiple windows fail closed even if a title uniquely matches.
  Control maps to Control and Alt maps to Option. Command shortcuts are not currently
  in the cross-platform action model. Combinations support modifiers plus one key.
- **Linux x64:** X11/Xorg, an EWMH window manager and xdotool. DEB declares xdotool;
  AppImage users install it separately. Wayland is rejected, including XWayland under
  a Wayland session: it cannot verify foreground across all native Wayland clients.

Window targeting does not identify tabs, panes or focused child controls. Keep the
intended terminal pane active in the selected window. A successful OS input call does
not confirm that an application acted on the input. No backend bypasses secure input,
OS foreground restrictions, or permissions. Focus changes and input injection cannot
be made atomic across an external application; checks bound this race but cannot
eliminate OS/application behavior outside this process.

## What this release verifies

- Windows local fmt/clippy/tests and frontend lint/build.
- CI fmt/clippy/tests and native bundle builds on Windows, macOS, Ubuntu 22.04.
- Pure/mock tests for input ordering, no input after failed focus verification,
  repeated Enter safety, ambiguous cached targets, explicit newline rejection,
  timestamp preservation, scheduler recovery/Skip and transactional state updates.
- Windows tests inspect event construction and query process identity. They do not
  inject keyboard input into a developer's desktop.
- CI run 35696978747: Windows 101 tests, macOS/Linux 92 each, all native builds passed.
  Dedicated isolated Windows Tk and Linux Xvfb/xterm probes confirmed exact text and
  a single Enter submission with no extra submission over two seconds.

Manual Windows 10/11 terminal, macOS Accessibility, Linux X11 desktop, suspend/resume
and installer-upgrade checks remain environment-dependent. Build success must never
be presented as evidence that all such manual scenarios passed. Record completed
manual checks with OS, terminal, privilege level and expected/actual Enter counts.

## Troubleshooting

| Symptom | Check |
| --- | --- |
| Text appears but Enter behaves differently | Confirm the active pane, use explicit PressKey Enter, release modifiers, inspect terminal-specific keybindings |
| Unexpected second Enter | Review existing task actions; Continue + Confirm and Double Enter intentionally submit twice |
| No input / TargetLostFocus | Keep the target foreground; close duplicate matching windows or refine the title |
| Windows InputInjectionFailed | Run the terminal unelevated, check desktop is unlocked; UIPI can block injection without identifying itself |
| macOS AutomationUnavailable | Grant Accessibility and Automation > System Events permission; keep one accessible target window |
| Linux AutomationUnavailable | Use X11/Xorg, install xdotool; do not use a Wayland session |
| Startup error shown | Preserve store.json and .bak; check filesystem access or use the app version supporting that schema |

## Primary references

- [Microsoft SendInput](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendinput): foreground stream, UIPI, held keyboard state.
- [Microsoft KEYBDINPUT](https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-keybdinput): physical scan codes, Unicode and extended flags.
- [Apple automation permissions](https://support.apple.com/guide/mac-help/mchl108e1718/mac).
- [Tauri Windows installers](https://v2.tauri.app/distribute/windows-installer/).
- [Tauri macOS bundles](https://v2.tauri.app/distribute/macos-application-bundle/).
- [Tauri CI packaging](https://v2.tauri.app/distribute/pipelines/github/).
