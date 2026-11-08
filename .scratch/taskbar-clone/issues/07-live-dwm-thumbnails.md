# 07: Live DWM Window Hover Thumbnails

**What to build:** Hardware-accelerated, zero-copy live window thumbnail previews that appear when hovering over a taskbar item, allowing users to visually inspect and select from open window instances.

**Blocked by:** 06: Task Grouping & Window Activation Controls

**Status:** closed

- [x] Displays a preview popup window when hovering over a task item representing one or more open windows.
- [x] Connects to the Desktop Window Manager thumbnail composition API to stream live window contents into the preview area without taking CPU screenshots.
- [x] For grouped tasks with multiple windows, renders individual preview tiles for each open window instance with its window title and close button.
- [x] Clicking a specific thumbnail preview immediately focuses and restores that window instance.
- [x] Unregisters and releases the DWM thumbnail composition cleanly when the hover ends or the window closes.

## Comments

### Resolution
- Merged branch `ticket-07` into `feat/taskbar-clone` at merge commit `d429d34` (source commit `dc95c48`).
- Implemented `src/dwm.rs` with the following architectural components:
  - `DwmThumbnailSession`: RAII lifecycle management of Win32 `DWM_THUMBNAIL` handles (`isize`) with automatic unregistration on `Drop` via `DwmUnregisterThumbnail`, tracking destination HWND, source HWND, and active destination rectangle.
  - `DwmThumbnailConfig`: Configurable composition parameters (destination rect, source crop rect, opacity, visibility, client-area-only flags) applied via `update_dwm_thumbnail` (`DwmUpdateThumbnailProperties`).
  - `calculate_thumbnail_aspect_fit`: Aspect-ratio preserving geometry calculation fitting arbitrary source window aspect ratios into preview tile bounds without distortion.
  - `PreviewTile`: Tile coordinate layout computation for single- and multi-window task groups, partitioning popup area into title header, close button, and thumbnail composition viewport.
  - `PreviewPopup`: Native Win32 preview popup window (`WS_POPUP`, `WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE`) positioned adjacent to taskbar items across all 4 dock edges (`Bottom`, `Top`, `Left`, `Right`) with multi-monitor work area clamping.
  - `PreviewPopupController`: Controller coordinating hover state, active popup lifecycle, tile selection, and DWM thumbnail session synchronization.
  - `PreviewClickAction`: Hit-testing actions (`FocusWindow`, `CloseWindow`, `Dismiss`, `None`) mapping click events on preview tiles to window activation or close commands.
- Comprehensive integration tests in `tests/dwm_test.rs`:
  - `test_thumbnail_session_zero_handle`
  - `test_thumbnail_session_lifecycle_and_drop`
  - `test_thumbnail_properties_config`
  - `test_thumbnail_aspect_ratio_fitting`
  - `test_preview_tile_layout_single_window`
  - `test_preview_tile_layout_multi_window`
  - `test_preview_popup_docking_all_edges`
  - `test_preview_popup_click_actions`
  - `test_preview_popup_controller_lifecycle`
  - `test_real_dwm_thumbnail_registration_with_mock_or_test_windows`
  - `test_real_preview_popup_native_window_lifecycle`
- Verification:
  - `cargo test`: All 83 tests passing across 12 test suites (11/11 DWM tests pass).
  - `cargo clippy --all-targets`: Passed cleanly with 0 warnings.
