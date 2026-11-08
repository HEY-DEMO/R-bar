# 01: Minimal Taskbar Loop & Crash Watchdog

**What to build:** The core application loop that spawns a visible taskbar window on the primary screen, safely hides the native Windows 11 taskbar window, installs a global panic watchdog to restore the native taskbar on abnormal exit, and provides a standalone recovery command (`--restore`) to unhide the native taskbar on demand.

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [ ] Launches a visible, hardware-accelerated taskbar window docked to the primary display.
- [ ] Finds and programmatically conceals the native Windows 11 taskbar window (`Shell_TrayWnd`) and secondary monitor trays while running.
- [ ] Installs a panic watchdog hook so any unexpected process panic immediately unhides and restores `Shell_TrayWnd`.
- [ ] Provides a `--restore` command-line flag that unhides the native taskbar and exits immediately without running the UI.
- [ ] Restores the native taskbar cleanly when the application terminates normally.

