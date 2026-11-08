# Spec: Windows 11 Taskbar Clone

**Status:** completed

## Problem Statement

Windows 11 removed core taskbar customization features that users relied on for years: the taskbar is locked permanently to the bottom screen edge, task grouping cannot be fully customized, deep visual personalization (such as frosted glass blur, classic beveled styling, or pure OLED black) is unsupported, and heavy WebView-based or closed-source alternatives consume excessive memory (150MB–250MB+) and introduce lag to daily desktop navigation.

## Solution

A high-performance, lightweight (under 30MB idle RAM), native Rust-based taskbar application that replaces the native Windows 11 taskbar visually while co-existing with `explorer.exe`. It reserves screen work area as a registered Win32 AppBar on any screen edge (`Bottom`, `Top`, `Left`, `Right`), offers live DWM window hover thumbnails, groups running tasks by application identifier, includes native hardware status widgets (Clock, Battery, CoreAudio Volume slider), provides pluggable themes (Glossy Acrylic, Material Flat, Retro Classic, OLED Minimalist), and features failsafe restoration hooks to guarantee desktop stability.

## User Stories

1. As a Windows user, I want the native Windows 11 taskbar hidden while the app is running, so that I have a clean, unified replacement interface.
2. As a user, I want the native Windows 11 taskbar to instantly unhide if the app exits or crashes, so that my desktop never becomes unresponsive or unusable.
3. As a user, I want a dedicated CLI command (`--restore`) to unhide the native taskbar at any time, so that I can recover my desktop immediately if needed.
4. As a user, I want the taskbar to dock to the Bottom, Top, Left, or Right edge of my monitor, so that I can arrange my workspace to match my preferences.
5. As a user with multiple monitors, I want an AppBar instance created on each connected display, so that I can manage windows seamlessly regardless of which screen I am looking at.
6. As a user, I want the desktop work area to shrink to accommodate the taskbar, so that maximized windows never overlap or hide behind the bar.
7. As a user, I want to select between four visual themes (Glossy Acrylic, Material Flat, Retro Classic, and OLED Minimalist), so that the taskbar matches my personal desktop aesthetic.
8. As a user, I want to configure custom background colors, blur translucency, and corner radii in a `config.toml` file, so that I can fine-tune theme details without recompiling.
9. As a user, I want running applications to appear on the taskbar with their correct icons and titles, so that I can quickly identify my open work.
10. As a user, I want background and helper windows (telemetry sinks, invisible message pumps, tooltips) filtered out, so that the taskbar remains clutter-free and memory-efficient.
11. As a user, I want multiple windows of the same application grouped under a single icon, so that my taskbar does not get overcrowded when opening dozens of windows.
12. As a user, I want to hover over a grouped task item to see live, hardware-accelerated DWM thumbnail previews of each window instance, so that I can visually pick the exact window I want.
13. As a user, I want clicking a task item to bring that window to the foreground (or restore it if minimized), so that I can switch tasks effortlessly.
14. As a user, I want clicking an already active task item to minimize it, so that I can quickly clear my screen.
15. As a user, I want to right-click a task item to access standard window controls (Restore, Minimize, Maximize, Close), so that I can manage windows directly from the bar.
16. As a user, I want to pin my favorite applications to the taskbar, so that I can launch them with a single click even when they are not running.
17. As a user, I want to click the Start button to invoke the native Windows 11 Start Menu, so that I retain full access to Windows search and pinned apps.
18. As a user, I want to view the current system time and date formatted clearly on the taskbar, so that I can keep track of time.
19. As a user, I want to view real-time battery percentage and AC charging status on laptops, so that I know my battery level without opening settings.
20. As a user, I want an interactive master audio volume slider, so that I can adjust or mute system audio directly from the bar.
21. As a user, I want a toggle button to temporarily unhide the native notification tray, so that I can interact with background utility icons (e.g. OneDrive, Discord, Steam) when needed.
22. As a user playing fullscreen 3D games or watching borderless videos, I want the taskbar to automatically yield its topmost z-order, so that it never obscures my media or game HUD.
23. As a user moving between displays with different scaling factors (e.g. 4K at 150% and 1080p at 100%), I want the taskbar to adapt dynamically via Per-Monitor V2 DPI awareness, so that icons and text stay sharp.
24. As a user, I want icon loading to happen asynchronously in the background, so that a slow or frozen external program never stutters the taskbar's UI framerate.
25. As a user, I want an optional Auto-Hide mode that conceals the taskbar until I hover over the screen edge, so that I can maximize vertical screen space on laptops.
26. As a user using Virtual Desktops, I want the taskbar to only show tasks belonging to the active virtual desktop, so that separate workspaces remain cleanly organized.

## Implementation Decisions

- **Native Shell Co-existence**: The app will run alongside `explorer.exe` rather than replacing the Windows logon shell. It programmatically hides `Shell_TrayWnd` and `Shell_SecondaryTrayWnd` via Win32 `ShowWindow(SW_HIDE)` and registers as an AppBar using `SHAppBarMessage(ABM_NEW)`.
- **UI Framework**: Slint UI with native GPU-accelerated rendering and declarative markup, keeping idle memory usage under 30MB RAM.
- **Window Lifecycle & Qualification**: Window tracking is powered by Win32 Shell Hooks (`RegisterShellHookWindow`) and WinEvents. Windows must pass qualification: `WS_VISIBLE`, not `WS_EX_TOOLWINDOW` unless `WS_EX_APPWINDOW`, unowned (`GW_OWNER == 0`), and uncloaked (`DwmGetWindowAttribute(DWMWA_CLOAKED) == 0`).
- **Live Previews**: Zero-copy, GPU-composited previews generated via `DwmRegisterThumbnail` and `DwmUpdateThumbnailProperties`.
- **Off-Thread Icon Pipeline**: Application icon extraction (`WM_GETICON`, `GetClassLongPtrW`, `ExtractIconExW`) runs on a dedicated background worker thread pool with an in-memory LRU cache to prevent UI thread blocking.
- **Dedicated Win32 Message Pump**: An invisible Win32 message window (`HWND_MESSAGE`) handles shell hooks, display configuration changes (`WM_DISPLAYCHANGE`), and power broadcasts, dispatching state changes to Slint via `slint::invoke_from_event_loop`.
- **Theming Engine**: Built-in support for four presets (`Glossy`, `Material`, `Retro`, `OLED`). `Glossy` integrates with Windows 11 DWM backdrop blur (`DwmSetWindowAttribute(DWMWA_SYSTEMBACKDROP_TYPE)`).
- **Configuration**: User settings, theme selections, docking edge, and pinned items persist in a human-editable TOML configuration file in the user's application data directory via `serde` and `toml`.
- **Crash Watchdog**: Global panic hook (`std::panic::set_hook`) and signal handlers restore `Shell_TrayWnd` visibility on any crash or normal exit.

## Testing Decisions

- **Good Test Philosophy**: Tests must exercise external domain behavior (state transformations, event handling, geometry math, config serialization), never low-level C function mock mechanics.
- **State & Event Seam**: Tested by feeding synthetic `ShellEvent` streams into the state manager and asserting resulting `TaskbarState` (grouping, active task, geometry rects).
- **Geometry & Docking Seam**: Tested by verifying AppBar rectangle calculations across all 4 edges (`Bottom`, `Top`, `Left`, `Right`) against arbitrary monitor dimensions.
- **Config Seam**: Tested by verifying TOML serialization/deserialization, fallback defaults, and malformed input resilience.
- **UI Seam**: Tested by verifying Slint component property bindings and layout responsiveness.

## Out of Scope

- Full replacement of `explorer.exe` as the Windows logon shell in the Windows Registry.
- Complete reimplementation of the Windows 11 Start Menu and Quick Settings flyouts from scratch in v1 (native flyouts are triggered instead).
- Reverse-engineering the Windows 11 22H2+ private XAML system tray into a custom in-process host (native tray toggle is provided instead).
- Taskbar extensions for macOS or Linux (strictly Windows 11 / Windows 10 1809+).

## Further Notes

- The binary will embed a Windows application manifest declaring `dpiAwarenessPerMonitorV2` and Common Controls v6.
- The project will be built with Rust 2021 edition and compiled using the native Slint build tools.

