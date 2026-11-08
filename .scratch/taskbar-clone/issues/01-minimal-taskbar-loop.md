# 01: Minimal Taskbar Loop & Crash Watchdog

**What to build:** The core application loop that spawns a visible taskbar window on the primary screen, safely hides the native Windows 11 taskbar window, installs a global panic watchdog to restore the native taskbar on abnormal exit, and provides a standalone recovery command (`--restore`) to unhide the native taskbar on demand.

**Blocked by:** None (can start immediately)

**Status:** closed

- [x] Launches a visible, hardware-accelerated taskbar window docked to the primary display.
- [x] Finds and programmatically conceals the native Windows 11 taskbar window (`Shell_TrayWnd`) and secondary monitor trays while running.
- [x] Installs a panic watchdog hook so any unexpected process panic immediately unhides and restores `Shell_TrayWnd`.
- [x] Provides a `--restore` command-line flag that unhides the native taskbar and exits immediately without running the UI.
- [x] Restores the native taskbar cleanly when the application terminates normally.

## Comments

### Resolution
- Merged branch `ticket-01` into `feat/taskbar-clone` at merge commit `913e771` (source commit `b55f135`).
- Implemented Slint UI taskbar window docked to display bottom, `hide_native_taskbar`, `restore_native_taskbar`, panic hook watchdog in `src/shell.rs`, and `--restore` CLI argument in `src/main.rs`.
- Validated with integration tests in `tests/shell_test.rs` and `tests/config_test.rs`. All tests pass.
