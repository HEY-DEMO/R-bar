# 11: Fullscreen Yield, Auto-Hide & Native Tray Toggle

**What to build:** Ergonomic and distraction-free operation, automatically suppressing the taskbar's top-most z-order when fullscreen games or borderless media players are active, providing an optional mouse-hover auto-hide slide, and offering a quick toggle button to surface the native Windows notification tray.

**Blocked by:** 04: System Status Module (Clock, Battery & Audio Volume), 10: Multi-Monitor Topology & AppBar Work Area Reservation

**Status:** resolved

- [x] Detects when an active foreground window covers the entire display area (e.g. 3D games, media players in fullscreen) and lowers the taskbar's z-order or conceals it so the game HUD is never obscured.
- [x] Supports an optional Auto-Hide configuration mode: taskbar conceals itself off-screen when inactive and smoothly animates into view when the mouse hovers over the screen edge.
- [x] Provides a toggle button on the taskbar that temporarily unhides or floats the native Windows notification tray (`TrayNotifyWnd`) when access to third-party background icons is needed.
- [x] Re-engages normal topmost docking when fullscreen applications lose focus or exit.
