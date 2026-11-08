# 03: Start Menu Trigger & Pinned App Launcher

**What to build:** Application launching capabilities on the taskbar, including a Start button that triggers the native Windows 11 Start Menu flyout, and interactive pinned application icons configured in the user settings that launch target programs on click.

**Blocked by:** 02: TOML Configuration & 4-Edge Geometry Engine

**Status:** closed

- [x] Displays a dedicated Start button with the Windows logo on the taskbar.
- [x] Clicking the Start button synthesizes the native Windows key event to open the native Windows 11 Start Menu and search interface.
- [x] Reads pinned application definitions (name, executable path, arguments, working directory) from the configuration file.
- [x] Renders pinned application icons in the launcher section of the bar.
- [x] Clicking a pinned application icon spawns the target application process asynchronously.

## Comments

### Resolution
- Merged branch `ticket-03` into `feat/taskbar-clone` at commit `397bb39` (source commit `05ed7c9`).
- Implemented `src/launcher.rs`:
  - `trigger_start_menu()` synthesizing native Windows key (`VK_LWIN`) keydown and keyup events via Win32 `SendInput` to toggle the native Windows 11 Start Menu and Search flyout.
  - `launch_app()` and `build_command()` asynchronously spawning pinned executables with arguments and custom working directories using `std::process::Command`.
  - Command argument tokenizer `parse_args()` supporting whitespace separation and single/double quoted parameters.
- Integrated configuration model:
  - Extended `src/config.rs` with `PinnedItem` and default pinned applications (File Explorer and Settings).
- Connected Slint UI in `src/main.rs` and `ui/taskbar.slint`:
  - Start button with Windows 11 logo vector rendering, hover animations, and click handlers dispatching `start_clicked`.
  - Pinned applications bar component binding config items to UI models, rendering icon buttons with tooltips, and dispatching `launch_app(index)` on click.
- Added comprehensive unit and integration tests in `tests/launcher_test.rs` (8 tests passing).
- Verified full test suite (`cargo test` passes all tests across all integration suites).
