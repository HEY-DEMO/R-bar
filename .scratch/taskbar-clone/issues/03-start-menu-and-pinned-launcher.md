# 03: Start Menu Trigger & Pinned App Launcher

**What to build:** Application launching capabilities on the taskbar, including a Start button that triggers the native Windows 11 Start Menu flyout, and interactive pinned application icons configured in the user settings that launch target programs on click.

**Blocked by:** 02: TOML Configuration & 4-Edge Geometry Engine

**Status:** ready-for-agent

- [ ] Displays a dedicated Start button with the Windows logo on the taskbar.
- [ ] Clicking the Start button synthesizes the native Windows key event to open the native Windows 11 Start Menu and search interface.
- [ ] Reads pinned application definitions (name, executable path, arguments, working directory) from the configuration file.
- [ ] Renders pinned application icons in the launcher section of the bar.
- [ ] Clicking a pinned application icon spawns the target application process asynchronously.

