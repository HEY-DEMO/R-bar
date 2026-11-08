# 05: Shell Hook Window Tracker & Qualification Filter

**What to build:** An event-driven window tracking pipeline powered by Win32 Shell Hooks and window messages that filters out background and system helper windows, displaying real-time task items for active and minimized user applications on the bar.

**Blocked by:** 01: Minimal Taskbar Loop & Crash Watchdog

**Status:** ready-for-agent

- [ ] Runs an isolated Win32 message pump listening for shell hook messages (`HSHELL_WINDOWCREATED`, `HSHELL_WINDOWDESTROYED`, `HSHELL_ACTIVATESHELLWINDOW`, `HSHELL_REDRAW`).
- [ ] Performs initial window enumeration on startup to populate currently running applications.
- [ ] Applies the canonical window qualification algorithm: must be visible, must not have tool window styles (unless explicit app window), must not be owned by another window, and must not be cloaked by the Desktop Window Manager.
- [ ] Accurately tracks when a window is focused, minimized, or restored and updates the task item state on the bar.
- [ ] Removes task items immediately when their corresponding application window is closed or destroyed.

