# 10: Multi-Monitor Topology & AppBar Work Area Reservation

**What to build:** True desktop integration across multiple displays, registering each taskbar instance as a Win32 Application Desktop Toolbar (AppBar) so maximized application windows automatically respect the bar's reserved screen space, and adapting dynamically to display resolution and topology changes.

**Blocked by:** 02: TOML Configuration & 4-Edge Geometry Engine, 05: Shell Hook Window Tracker & Qualification Filter

**Status:** closed

- [x] Discovers all connected display monitors using display enumeration APIs.
- [x] Spawns a dedicated taskbar AppBar instance on each connected monitor matching the configured edge.
- [x] Registers desktop work area reservations via `SHAppBarMessage` (`ABM_NEW`, `ABM_QUERYPOS`, `ABM_SETPOS`) so maximized windows on any monitor shrink to fit around the bar.
- [x] Unregisters AppBar reservations cleanly upon exit or monitor disconnection (`ABM_REMOVE`).
- [x] Listens for display topology changes (`WM_DISPLAYCHANGE`) and DPI scale adjustments (`WM_DPICHANGED`), dynamically recalculating bar heights and work areas.

## Comments

### Resolution
- Merged branch `ticket-10` into `feat/taskbar-clone` at commit `43891b4` (source commit `89b6706`).
- Implemented `src/appbar.rs`:
  - `MonitorInfo`: monitor enumeration via `EnumDisplayMonitors` querying display rects, work areas, DPI scaling, and primary display flags.
  - `AppBarRegistration`: Win32 Application Desktop Toolbar lifecycle managing `SHAppBarMessage` (`ABM_NEW`, `ABM_QUERYPOS`, `ABM_SETPOS`, `ABM_REMOVE`), reserving desktop work area on all 4 edges (Bottom, Top, Left, Right) so maximized windows shrink to fit around the bar.
  - `MonitorBar`: per-monitor taskbar management tracking monitor metadata, HWND, and active AppBar reservation.
  - `MultiMonitorManager`: lifecycle coordination across multi-monitor topologies, dynamically spawning monitor bars, handling `WM_DISPLAYCHANGE` and `WM_DPICHANGED` events, re-enumerating displays, and recalculating bar dimensions and work area reservations.
- Added comprehensive unit and integration tests in `tests/appbar_test.rs`:
  - Testing monitor enumeration, 4-edge positioning calculations, primary and offset monitor geometry, AppBar registration lifecycle on real Win32 windows, and display/DPI change event handling.
- Verified all 58 tests pass via `cargo test`.
