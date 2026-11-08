# 10: Multi-Monitor Topology & AppBar Work Area Reservation

**What to build:** True desktop integration across multiple displays, registering each taskbar instance as a Win32 Application Desktop Toolbar (AppBar) so maximized application windows automatically respect the bar's reserved screen space, and adapting dynamically to display resolution and topology changes.

**Blocked by:** 02: TOML Configuration & 4-Edge Geometry Engine, 05: Shell Hook Window Tracker & Qualification Filter

**Status:** ready-for-agent

- [ ] Discovers all connected display monitors using display enumeration APIs.
- [ ] Spawns a dedicated taskbar AppBar instance on each connected monitor matching the configured edge.
- [ ] Registers desktop work area reservations via `SHAppBarMessage` (`ABM_NEW`, `ABM_QUERYPOS`, `ABM_SETPOS`) so maximized windows on any monitor shrink to fit around the bar.
- [ ] Unregisters AppBar reservations cleanly upon exit or monitor disconnection (`ABM_REMOVE`).
- [ ] Listens for display topology changes (`WM_DISPLAYCHANGE`) and DPI scale adjustments (`WM_DPICHANGED`), dynamically recalculating bar heights and work areas.

