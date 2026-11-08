# Taskbar Clone

A lightweight, native Rust-based replacement for the Windows 11 taskbar that runs alongside the Windows shell.

## Language

**AppBar**:
A registered desktop toolbar window that reserves dedicated screen space from the Windows desktop work area.
_Avoid_: Dock, panel, desktop margin

**Native Shell**:
The default desktop user interface provided by Windows Explorer (`explorer.exe`), including `Shell_TrayWnd`.
_Avoid_: Windows OS shell, desktop window

**Task Item**:
A visual button representing an active or minimized top-level application window tracked on the bar.
_Avoid_: App button, window tab, running process

**Pinned Item**:
A persistent application shortcut anchored to the taskbar regardless of whether its process is currently running.
_Avoid_: Static shortcut, launcher icon, fixed app

**Grouped Task**:
A task item that aggregates multiple window instances belonging to the same application under a single icon.
_Avoid_: Window cluster, tab stack

**Flyout**:
A temporary popup pane invoked from the taskbar for system controls (Start Menu, Quick Settings, Calendar).
_Avoid_: Dropdown, system popup, drawer

**Shell Hook**:
A Win32 subsystem hook used to monitor top-level window lifecycle events (creation, destruction, activation).
_Avoid_: Window listener, event observer

**Window Qualification**:
The rule set (visibility, window style, ownership, DWM cloaking) determining whether an OS window represents a user-facing task.
_Avoid_: Window filtering, process filtering

**DWM Thumbnail**:
A hardware-accelerated live preview of a window composited directly by the Desktop Window Manager into an application window.
_Avoid_: Screenshot preview, window capture

**Icon Pipeline**:
An asynchronous background worker system for resolving, decoding, and caching application icon bitmaps without blocking UI rendering.
_Avoid_: Icon fetcher, icon loader

**Status Indicator**:
A dedicated taskbar component displaying real-time hardware or system metrics (e.g. clock, volume, battery).
_Avoid_: Widget, tray icon, notification item

**Fullscreen Yield**:
The automatic suppression or lowering of the taskbar's top-most z-order when a foreground application covers the entire display area.
_Avoid_: Game mode, auto-hide on game, fullscreen bypass

**Bar Theme**:
A visual skin preset (e.g. Glossy Acrylic, Material Flat, Retro, Minimalist OLED) controlling DWM backdrop effects, surface transparency, corner radius, and accent styling.
_Avoid_: Style sheet, skin file, skin pack

**Dock Edge**:
The physical display monitor boundary (`Bottom`, `Top`, `Left`, `Right`) to which the AppBar is anchored and where work area reservations apply.
_Avoid_: Screen side, bar position

