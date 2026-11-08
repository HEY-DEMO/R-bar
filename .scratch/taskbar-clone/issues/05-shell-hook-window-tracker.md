# 05: Shell Hook Window Tracker & Qualification Filter

**What to build:** An event-driven window tracking pipeline powered by Win32 Shell Hooks and window messages that filters out background and system helper windows, displaying real-time task items for active and minimized user applications on the bar.

**Blocked by:** 01: Minimal Taskbar Loop & Crash Watchdog

**Status:** closed

- [x] Runs an isolated Win32 message pump listening for shell hook messages (`HSHELL_WINDOWCREATED`, `HSHELL_WINDOWDESTROYED`, `HSHELL_ACTIVATESHELLWINDOW`, `HSHELL_REDRAW`).
- [x] Performs initial window enumeration on startup to populate currently running applications.
- [x] Applies the canonical window qualification algorithm: must be visible, must not have tool window styles (unless explicit app window), must not be owned by another window, and must not be cloaked by the Desktop Window Manager.
- [x] Accurately tracks when a window is focused, minimized, or restored and updates the task item state on the bar.
- [x] Removes task items immediately when their corresponding application window is closed or destroyed.

## Comments

### Resolution
- Merged branch `ticket-05` into `feat/taskbar-clone` at commit `4bf5c4e` (source commit `9758efe`).
- Implemented `src/events.rs`:
  - Isolated background Win32 message pump listening for shell hook messages (`RegisterShellHookWindow`, `HSHELL_WINDOWCREATED`, `HSHELL_WINDOWDESTROYED`, `HSHELL_ACTIVATESHELLWINDOW`, `HSHELL_REDRAW`, `HSHELL_WINDOWACTIVATED`, `HSHELL_RUDEAPPACTIVATED`).
  - Thread-safe event queue and crossbeam channel dispatch with graceful shutdown via `WM_CLOSE` / `WM_QUIT`.
- Implemented `src/tasks.rs`:
  - Canonical window qualification criteria in `qualify_window` adhering to Windows Shell taskbar rules: `IsWindowVisible`, `WS_EX_TOOLWINDOW` filtering unless overridden by `WS_EX_APPWINDOW`, unowned window validation (`GetWindow(GW_OWNER)`), and DWM cloaking verification (`DwmGetWindowAttribute(DWMWA_CLOAKED)`).
  - Initial startup window enumeration (`enumerate_initial_windows`) using `EnumWindows`.
  - Window state tracker (`TaskManager`) with real-time tracking for window creation, destruction, focus changes, activation, and window state inspection (`WindowState`: Active, Normal, Minimized).
  - Window title, process ID, and process executable name retrieval for grouping and identification.
- Added comprehensive unit and integration tests in `tests/events_test.rs` and `tests/tasks_test.rs`.
- Full project test suite verified (`cargo test` passing all 26 tests across all 6 test suites).
