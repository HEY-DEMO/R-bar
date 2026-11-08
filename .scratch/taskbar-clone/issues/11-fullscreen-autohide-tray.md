# 11: Fullscreen Yield, Auto-Hide & Native Tray Toggle

**What to build:** Ergonomic and distraction-free operation, automatically suppressing the taskbar's top-most z-order when fullscreen games or borderless media players are active, providing an optional mouse-hover auto-hide slide, and offering a quick toggle button to surface the native Windows notification tray.

**Blocked by:** 04: System Status Module (Clock, Battery & Audio Volume), 10: Multi-Monitor Topology & AppBar Work Area Reservation

**Status:** closed

- [x] Detects when an active foreground window covers the entire display area (e.g. 3D games, media players in fullscreen) and lowers the taskbar's z-order or conceals it so the game HUD is never obscured.
- [x] Supports an optional Auto-Hide configuration mode: taskbar conceals itself off-screen when inactive and smoothly animates into view when the mouse hovers over the screen edge.
- [x] Provides a toggle button on the taskbar that temporarily unhides or floats the native Windows notification tray (`TrayNotifyWnd`) when access to third-party background icons is needed.
- [x] Re-engages normal topmost docking when fullscreen applications lose focus or exit.

## Comments

### Resolution
- Merged branch `ticket-11` into `feat/taskbar-clone` at merge commit `3743310` (source commit `fc1d275`).
- Implemented `src/autohide.rs` and integrated with `MultiMonitorManager`, `events.rs`, `system.rs`, `appbar.rs`, and `ui/taskbar.slint`:
  - `is_window_fullscreen`: Determines whether a window covers or exceeds the monitor's total bounding rectangle (`bounds`) and is visible, unminimized, and non-desktop/shell.
  - `find_fullscreen_window_on_monitor`: Queries `GetForegroundWindow` and checks bounds against monitor geometry to detect fullscreen games, borderless windows, and media players.
  - `set_taskbar_topmost`: Dynamically adjusts taskbar HWND z-order using `SetWindowPos` (`HWND_TOPMOST` / `HWND_NOTOPMOST` / `HWND_BOTTOM`) with flags `SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE`.
  - `MultiMonitorManager::update_fullscreen_state`: Iterates active monitors, evaluates whether any foreground window occupies full screen, lowers taskbar z-order (`HWND_BOTTOM`) or conceals when fullscreen, and restores topmost status upon exit or focus change.
  - `AutoHideController`: Complete finite state machine (`Shown`, `Hiding`, `Hidden`, `Showing`) managing auto-hide transitions, hover edge detection (`is_cursor_at_edge`), delay timers, and slide offset calculation (`calculate_hidden_rect`).
  - `calculate_hidden_rect`: Computes hidden docking rect based on screen edge (`Bottom`, `Top`, `Left`, `Right`) leaving a configurable peek strip (default 2px) or concealing completely off-screen.
  - `toggle_native_tray`: Win32 native tray toggle locating `TrayNotifyWnd` inside `Shell_TrayWnd` and applying `ShowWindow(SW_SHOW | SW_RESTORE)` or `SW_HIDE`, setting foreground focus and topmost z-order.
  - Slint UI tray toggle integration: Added tray button with visual feedback, hover styling, and toggle callback in `ui/taskbar.slint`.
- Comprehensive integration tests in `tests/autohide_test.rs`:
  - `test_fullscreen_rect_detection_exact_match`
  - `test_fullscreen_rect_detection_borderless_oversized`
  - `test_fullscreen_rect_detection_floating_and_minimized`
  - `test_fullscreen_rect_detection_maximized_not_fullscreen`
  - `test_fullscreen_rect_detection_secondary_offset_monitor`
  - `test_live_window_fullscreen_null_and_desktop_safety`
  - `test_autohide_state_variants_and_transitions`
  - `test_is_cursor_at_edge_all_four_edges`
  - `test_is_cursor_in_rect`
  - `test_calculate_hidden_rect_all_four_edges_with_peek`
  - `test_calculate_hidden_rect_zero_peek_completely_offscreen`
  - `test_calculate_hidden_rect_offset_secondary_monitor`
  - `test_autohide_controller_edge_hover_lifecycle`
  - `test_shell_event_fullscreen_changed_variant`
  - `test_find_native_tray_notify_window_safety`
  - `test_toggle_native_tray_execution`
  - `test_taskbar_topmost_z_order_toggle`
  - `test_multi_monitor_manager_fullscreen_yield`
- Verification:
  - `cargo test`: All 101 tests pass across 13 test suites (18/18 autohide tests pass).
  - All features integrated into `feat/taskbar-clone`.
