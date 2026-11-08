# 06: Task Grouping & Window Activation Controls

**What to build:** Ergonomic task management on the bar, clustering multiple windows of the same application under a single task icon, with full click controls to activate, minimize, or close target windows.

**Blocked by:** 05: Shell Hook Window Tracker & Qualification Filter

**Status:** closed

- [x] Clusters multiple window instances belonging to the same application under one grouped task item by application identifier or process path.
- [x] Displays a visual count badge or multi-window indicator when a task item represents multiple windows.
- [x] Left-clicking an inactive task item brings that window to the foreground and restores it if minimized.
- [x] Left-clicking an already active task item minimizes that window.
- [x] Right-clicking a task item opens standard window commands (Minimize, Maximize, Restore, Close) via standard system commands.

## Comments

### Resolution
- Implemented task grouping helper methods and indicators on `GroupedTask` in `src/tasks.rs`:
  - `window_count(&self) -> usize`: Returns count of window instances in group for count badge display.
  - `has_multiple_windows(&self) -> bool`: Visual indicator check for multi-window groups.
  - `is_active(&self) -> bool`: Checks if any window in the group is active.
  - `active_window(&self) -> Option<&TaskItem>`: Returns currently focused window instance.
  - `primary_window(&self) -> Option<&TaskItem>`: Returns primary window (active or first window).
- Implemented `src/controls.rs` and exported in `src/lib.rs` and `src/tasks.rs`:
  - `WindowCommand`: Enum (`Restore`, `Minimize`, `Maximize`, `Close`) mapping to Win32 `WM_SYSCOMMAND` codes (`SC_RESTORE`, `SC_MINIMIZE`, `SC_MAXIMIZE`, `SC_CLOSE`).
  - `TaskClickAction`: Enum (`Activated(isize)`, `Minimized(isize)`, `None`).
  - `handle_task_click(task: &GroupedTask)`: Left-click handler toggling between activating an inactive window (bringing to foreground and restoring if minimized) and minimizing an already active window.
  - `handle_task_click_state(tasks: &mut [TaskItem], hwnd: isize)`: In-memory task state updater for click toggle operations.
  - `execute_window_command(hwnd: isize, cmd: WindowCommand)`: Direct execution of window management commands via Win32 `ShowWindow` and `PostMessageW(WM_SYSCOMMAND)`.
  - `open_window_system_menu(hwnd: isize, x: i32, y: i32)`: Populates and displays native Win32 system menu (`GetSystemMenu`, `TrackPopupMenu`).
- Comprehensive unit and integration test coverage in `tests/controls_test.rs`:
  - `test_grouped_task_window_count_and_indicators`
  - `test_task_click_toggle_behavior`
  - `test_task_click_state_mutation`
  - `test_window_commands_and_syscommand_codes`
  - `test_multi_window_grouping_clusters_and_controls`
- All tests pass (31 tests passing across 7 test suites), and `cargo clippy --all-targets` passes with 0 warnings.

