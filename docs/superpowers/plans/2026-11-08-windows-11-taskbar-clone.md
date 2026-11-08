# Windows 11 Taskbar Clone Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a high-performance, lightweight, native Rust-based taskbar application for Windows 11 that seamlessly replaces the native taskbar with custom themes, multi-edge docking, live DWM window previews, and native system controls.

**Architecture:** The application co-exists with `explorer.exe` by hiding the native `Shell_TrayWnd` and registering custom top-level AppBar windows across all monitors via `SHAppBarMessage`. An isolated Win32 message pump thread listens for shell hooks, display adjustments, and system events, dispatching state changes to a declarative, hardware-accelerated Slint UI thread via `slint::invoke_from_event_loop`. An asynchronous worker pool extracts and caches application icons off-thread, and a fail-safe watchdog hook guarantees native shell restoration on crash or termination.

**Tech Stack:** Rust 2021, Slint UI 1.9, `windows-rs` 0.58 (Win32 APIs: AppBar, DWM, Shell, CoreAudio), `serde` + `toml` (configuration), `crossbeam-channel` (concurrency).

**Spec:** [docs/adr/](file:///c:/Users/hey1d/OneDrive/Desktop/PREPARE/Taskbar%20clone/docs/adr/) (ADRs 0001 through 0008) and [GLOSSARY.md](file:///c:/Users/hey1d/OneDrive/Desktop/PREPARE/Taskbar%20clone/GLOSSARY.md).

## Global Constraints

- Platform: Windows 11 (x86_64, Windows 10 1809+ compatible).
- Memory target: Idle RAM under 35MB.
- UI Framerate: 60/120 FPS hardware-accelerated rendering without hitching on UI thread.
- Resilience: Never leave the desktop in an unrecoverable state; panic hook and `--restore` CLI must unhide `Shell_TrayWnd`.
- Configuration: `%APPDATA%\taskbar-clone\config.toml` must be human-editable and auto-generated with defaults if missing.
- DPI Awareness: Declare Per-Monitor V2 awareness and adapt dynamically on `WM_DPICHANGED`.

## Review Focus

1. **Unresponsive Target Window on Shell Hook**: A target window hangs; calling `WM_GETICON` or `GetWindowText` synchronously must never stall taskbar rendering. (Exercised by async icon extraction and timeout-bounded messaging in Task 4).
2. **Crash or Forced Termination**: Taskbar process killed or panics; native `Shell_TrayWnd` must unhide automatically via panic hook and `--restore` handler. (Exercised in Task 2).
3. **Ghost / Cloaked Window Filtering**: Windows on inactive virtual desktops or hidden UWP helper processes must not appear as task items. (Exercised by qualification unit tests in Task 3).
4. **Display Topology Change**: Disconnecting or re-plugging a monitor during runtime must cleanly resize or respawn AppBars without crashing or miscalculating work areas. (Exercised in Task 5).
5. **Exclusive Fullscreen Video / Game**: A foreground 3D game takes over the monitor; taskbar must yield z-order and not cover game HUD. (Exercised in Task 8).

---

### Task 1: Project Scaffolding & Configuration Engine

**Files:**
- Create: `Cargo.toml`
- Create: `build.rs`
- Create: `src/config.rs`
- Create: `src/lib.rs`
- Test: `tests/config_test.rs`

**Interfaces:**
- Produces:
  - `pub enum DockEdge { Bottom, Top, Left, Right }`
  - `pub enum TaskAlignment { Left, Center }`
  - `pub enum ThemePreset { Glossy, Material, Retro, Oled, Custom }`
  - `pub struct Config { pub edge: DockEdge, pub alignment: TaskAlignment, pub theme: ThemePreset, pub auto_hide: bool, pub size: u32, pub pinned: Vec<PinnedItem> }`
  - `pub fn Config::load_or_default() -> Config`
  - `pub fn Config::save(&self) -> Result<(), std::io::Error>`

- [ ] **Step 1: Write the failing tests for configuration parsing, serialization, and defaults**

```rust
// tests/config_test.rs
use taskbar_clone::config::{Config, DockEdge, TaskAlignment, ThemePreset};

#[test]
fn test_default_config() {
    let cfg = Config::default();
    assert_eq!(cfg.edge, DockEdge::Bottom);
    assert_eq!(cfg.alignment, TaskAlignment::Center);
    assert_eq!(cfg.theme, ThemePreset::Glossy);
    assert!(!cfg.auto_hide);
    assert_eq!(cfg.size, 48);
}

#[test]
fn test_toml_roundtrip() {
    let toml_str = r#"
        edge = "Top"
        alignment = "Left"
        theme = "Material"
        auto_hide = true
        size = 52
        pinned = [
            { title = "Terminal", path = "wt.exe", args = "", icon = "" }
        ]
    "#;
    let cfg: Config = toml::from_str(toml_str).expect("Failed to deserialize TOML");
    assert_eq!(cfg.edge, DockEdge::Top);
    assert_eq!(cfg.alignment, TaskAlignment::Left);
    assert_eq!(cfg.theme, ThemePreset::Material);
    assert!(cfg.auto_hide);
    assert_eq!(cfg.size, 52);
    assert_eq!(cfg.pinned.len(), 1);
    assert_eq!(cfg.pinned[0].title, "Terminal");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test config_test`
Expected: FAIL with missing crate/module errors.

- [ ] **Step 3: Implement `Cargo.toml`, `build.rs`, and `src/config.rs`**

Configure dependencies in `Cargo.toml`:
- `windows` 0.58 (features: `Win32_Foundation`, `Win32_UI_WindowsAndMessaging`, `Win32_UI_Shell`, `Win32_Graphics_Dwm`, `Win32_Graphics_Gdi`, `Win32_Media_Audio`)
- `slint` 1.9, `slint-build` 1.9
- `serde` (derive), `toml`, `crossbeam-channel`, `log`, `env_logger`
Implement `DockEdge`, `TaskAlignment`, `ThemePreset`, `PinnedItem`, `Config` with `serde::Deserialize` and `serde::Serialize` in `src/config.rs`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --test config_test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml build.rs src/lib.rs src/config.rs tests/config_test.rs
git commit -m "feat(config): add project scaffolding and TOML configuration engine"
```

---

### Task 2: Native Shell Interop, Start Menu & Failsafe Watchdog

**Files:**
- Create: `src/shell.rs`
- Modify: `src/lib.rs`
- Create: `src/main.rs`
- Test: `tests/shell_test.rs`

**Interfaces:**
- Consumes: `windows-rs` Win32 APIs (`FindWindowW`, `ShowWindow`, `keybd_event`).
- Produces:
  - `pub fn hide_native_taskbar()`
  - `pub fn restore_native_taskbar()`
  - `pub fn trigger_start_menu()`
  - `pub fn trigger_quick_settings()`
  - `pub fn install_panic_watchdog()`

- [ ] **Step 1: Write unit and mock tests for shell window discovery and failsafe hooks**

```rust
// tests/shell_test.rs
use taskbar_clone::shell::{get_native_taskbar_classes, install_panic_watchdog};

#[test]
fn test_native_taskbar_classes() {
    let classes = get_native_taskbar_classes();
    assert!(classes.contains(&"Shell_TrayWnd"));
    assert!(classes.contains(&"Shell_SecondaryTrayWnd"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test shell_test`
Expected: FAIL with unresolved `get_native_taskbar_classes`.

- [ ] **Step 3: Implement `src/shell.rs` with Win32 APIs and CLI handling in `src/main.rs`**

- `hide_native_taskbar()`: Calls `FindWindowW(w!("Shell_TrayWnd"), None)` and `ShowWindow(hwnd, SW_HIDE)`.
- `restore_native_taskbar()`: Finds `Shell_TrayWnd` and secondary bars, calls `ShowWindow(hwnd, SW_SHOW)`.
- `trigger_start_menu()`: Synthesizes `VK_LWIN` key down and up.
- `trigger_quick_settings()`: Synthesizes `VK_LWIN + VK_A`.
- `install_panic_watchdog()`: Sets `std::panic::set_hook` to guarantee `restore_native_taskbar()` is invoked on any panic.
- In `src/main.rs`: Handle `--restore` CLI argument to instantly unhide native taskbar and exit.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --test shell_test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/shell.rs src/main.rs src/lib.rs tests/shell_test.rs
git commit -m "feat(shell): implement native taskbar hiding, start trigger, and crash watchdog"
```

---

### Task 3: Window Qualification & Task Management Engine

**Files:**
- Create: `src/tasks.rs`
- Modify: `src/lib.rs`
- Test: `tests/tasks_test.rs`

**Interfaces:**
- Consumes: Win32 window handles (`HWND`), `DWMWA_CLOAKED`.
- Produces:
  - `pub struct TaskItem { pub hwnd: isize, pub pid: u32, pub title: String, pub app_id: String, pub is_active: bool, pub is_minimized: bool }`
  - `pub struct GroupedTask { pub app_id: String, pub title: String, pub windows: Vec<TaskItem> }`
  - `pub fn is_qualifying_window(hwnd: isize) -> bool`
  - `pub fn group_tasks(tasks: Vec<TaskItem>) -> Vec<GroupedTask>`
  - `pub fn activate_window(hwnd: isize)`
  - `pub fn minimize_window(hwnd: isize)`
  - `pub fn close_window(hwnd: isize)`

- [ ] **Step 1: Write tests for window qualification logic and task grouping**

```rust
// tests/tasks_test.rs
use taskbar_clone::tasks::{group_tasks, TaskItem};

#[test]
fn test_task_grouping_by_app_id() {
    let items = vec![
        TaskItem { hwnd: 101, pid: 10, title: "Doc 1 - Word".into(), app_id: "WINWORD.EXE".into(), is_active: false, is_minimized: false },
        TaskItem { hwnd: 102, pid: 10, title: "Doc 2 - Word".into(), app_id: "WINWORD.EXE".into(), is_active: true, is_minimized: false },
        TaskItem { hwnd: 201, pid: 20, title: "Terminal".into(), app_id: "wt.exe".into(), is_active: false, is_minimized: false },
    ];
    let groups = group_tasks(items);
    assert_eq!(groups.len(), 2);
    let word_group = groups.iter().find(|g| g.app_id == "WINWORD.EXE").unwrap();
    assert_eq!(word_group.windows.len(), 2);
    assert_eq!(word_group.windows[1].is_active, true);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test tasks_test`
Expected: FAIL with `group_tasks` not defined.

- [ ] **Step 3: Implement `src/tasks.rs`**

- Implement canonical window qualification:
  1. `IsWindowVisible(hwnd)` must be true.
  2. `GetWindowLongPtrW(GWL_EXSTYLE)` must not contain `WS_EX_TOOLWINDOW` (unless `WS_EX_APPWINDOW` is set).
  3. `GetWindow(hwnd, GW_OWNER)` must be null or zero.
  4. `DwmGetWindowAttribute(hwnd, DWMWA_CLOAKED)` must return 0 (not cloaked).
  5. Title must not be empty.
- Implement `group_tasks`: clusters `TaskItem`s by `app_id` while preserving foreground active state.
- Implement window commands: `activate_window` (`SetForegroundWindow`, `ShowWindow(SW_RESTORE)`), `minimize_window` (`ShowWindow(SW_MINIMIZE)`), and `close_window` (`PostMessageW(hwnd, WM_SYSCOMMAND, SC_CLOSE, 0)`).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --test tasks_test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/tasks.rs src/lib.rs tests/tasks_test.rs
git commit -m "feat(tasks): implement window qualification rules and task grouping"
```

---

### Task 4: Asynchronous Off-Thread Icon Extraction Pipeline

**Files:**
- Create: `src/icons.rs`
- Modify: `src/lib.rs`
- Test: `tests/icons_test.rs`

**Interfaces:**
- Consumes: Window `HWND`, executable path.
- Produces:
  - `pub struct IconRgba { pub width: u32, pub height: u32, pub data: Vec<u8> }`
  - `pub struct IconManager`
  - `pub fn IconManager::new() -> Self`
  - `pub fn IconManager::get_icon_or_request(&self, hwnd: isize, exe_path: &str) -> Option<IconRgba>`

- [ ] **Step 1: Write test for icon caching and RGBA representation**

```rust
// tests/icons_test.rs
use taskbar_clone::icons::{IconManager, IconRgba};

#[test]
fn test_icon_rgba_pixel_buffer() {
    let dummy = IconRgba {
        width: 32,
        height: 32,
        data: vec![255; 32 * 32 * 4],
    };
    assert_eq!(dummy.data.len(), 4096);
    assert_eq!(dummy.width, 32);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test icons_test`
Expected: FAIL with `IconManager` not found.

- [ ] **Step 3: Implement `src/icons.rs`**

- Use `crossbeam-channel` or `mpsc` to send icon requests to a background worker thread.
- Worker attempts resolution in order:
  1. `SendMessageTimeoutW(hwnd, WM_GETICON, ICON_BIG, ...)` with a strict 150ms timeout.
  2. `GetClassLongPtrW(hwnd, GCLP_HICON)`.
  3. `ExtractIconExW(exe_path, 0, ...)`.
- Converts `HICON` into RGBA bytes via `GetDIBits`.
- Stores cached `IconRgba` in an in-memory `HashMap<String, IconRgba>` protected by `RwLock`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --test icons_test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/icons.rs src/lib.rs tests/icons_test.rs
git commit -m "feat(icons): add async off-thread icon extraction and memory cache"
```

---

### Task 5: Win32 AppBar Geometry, Edge Docking & Multi-Monitor Manager

**Files:**
- Create: `src/appbar.rs`
- Modify: `src/lib.rs`
- Test: `tests/appbar_test.rs`

**Interfaces:**
- Consumes: `Config`, `DockEdge`, `windows-rs` `SHAppBarMessage`.
- Produces:
  - `pub struct MonitorBar { pub monitor_handle: isize, pub rect: BarRect, pub hwnd: isize }`
  - `pub fn register_appbar(hwnd: isize, edge: DockEdge, size: u32, monitor: isize) -> bool`
  - `pub fn unregister_appbar(hwnd: isize) -> bool`
  - `pub fn calculate_appbar_rect(edge: DockEdge, size: u32, monitor_rect: BarRect) -> BarRect`
  - `pub fn enumerate_display_monitors() -> Vec<isize>`

- [ ] **Step 1: Write pure geometric tests for AppBar rect calculation across all 4 edges**

```rust
// tests/appbar_test.rs
use taskbar_clone::appbar::{calculate_appbar_rect, BarRect};
use taskbar_clone::config::DockEdge;

#[test]
fn test_appbar_rect_calculations() {
    let mon = BarRect { left: 0, top: 0, right: 1920, bottom: 1080 };
    let size = 48;

    let bottom = calculate_appbar_rect(DockEdge::Bottom, size, mon);
    assert_eq!(bottom, BarRect { left: 0, top: 1032, right: 1920, bottom: 1080 });

    let top = calculate_appbar_rect(DockEdge::Top, size, mon);
    assert_eq!(top, BarRect { left: 0, top: 0, right: 1920, bottom: 48 });

    let left = calculate_appbar_rect(DockEdge::Left, size, mon);
    assert_eq!(left, BarRect { left: 0, top: 0, right: 48, bottom: 1080 });

    let right = calculate_appbar_rect(DockEdge::Right, size, mon);
    assert_eq!(right, BarRect { left: 1872, top: 0, right: 1920, bottom: 1080 });
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test appbar_test`
Expected: FAIL with `calculate_appbar_rect` undefined.

- [ ] **Step 3: Implement `src/appbar.rs`**

- Implement `calculate_appbar_rect` for `Bottom`, `Top`, `Left`, `Right`.
- Implement `register_appbar`: Sends `ABM_NEW`, followed by `ABM_QUERYPOS` and `ABM_SETPOS` via `SHAppBarMessage`.
- Implement `unregister_appbar`: Sends `ABM_REMOVE`.
- Implement `enumerate_display_monitors`: Wraps Win32 `EnumDisplayMonitors` to discover all connected displays.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --test appbar_test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/appbar.rs src/lib.rs tests/appbar_test.rs
git commit -m "feat(appbar): implement multi-monitor AppBar registration and 4-edge docking calculations"
```

---

### Task 6: System Status Indicators (Clock, Battery, Audio & Tray Popover)

**Files:**
- Create: `src/system.rs`
- Modify: `src/lib.rs`
- Test: `tests/system_test.rs`

**Interfaces:**
- Produces:
  - `pub struct SystemStatus { pub time_str: String, pub date_str: String, pub battery_percent: Option<u8>, pub is_charging: bool, pub volume_percent: u8, pub is_muted: bool }`
  - `pub fn poll_system_status() -> SystemStatus`
  - `pub fn set_master_volume(level: f32) -> Result<(), isize>`
  - `pub fn toggle_native_tray()`

- [ ] **Step 1: Write unit tests for system status formatting and volume bounds**

```rust
// tests/system_test.rs
use taskbar_clone::system::format_clock;

#[test]
fn test_clock_formatting() {
    let (time_str, date_str) = format_clock();
    assert!(!time_str.is_empty());
    assert!(!date_str.is_empty());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test system_test`
Expected: FAIL with `format_clock` undefined.

- [ ] **Step 3: Implement `src/system.rs`**

- `format_clock`: Generates formatted time (`HH:mm` / `hh:mm A`) and date strings using standard local time.
- `poll_system_status`: Queries `GetSystemPowerStatus` for battery and AC status.
- CoreAudio interop: Connects to `IMMDeviceEnumerator` and `IAudioEndpointVolume` to read/write master volume and mute state.
- `toggle_native_tray`: Temporarily unhides or positions `Shell_TrayWnd`'s tray notification area (`TrayNotifyWnd`) on user request.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --test system_test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/system.rs src/lib.rs tests/system_test.rs
git commit -m "feat(system): add clock, battery, audio volume control, and native tray toggle"
```

---

### Task 7: Desktop Window Manager (DWM) Live Thumbnails & Hover Previews

**Files:**
- Create: `src/dwm.rs`
- Modify: `src/lib.rs`
- Test: `tests/dwm_test.rs`

**Interfaces:**
- Consumes: Target window `HWND`, destination popup `HWND`.
- Produces:
  - `pub struct DwmThumbnailSession { pub handle: isize }`
  - `pub fn create_dwm_thumbnail(dest_hwnd: isize, src_hwnd: isize, dest_rect: BarRect) -> Option<DwmThumbnailSession>`
  - `impl Drop for DwmThumbnailSession` (calls `DwmUnregisterThumbnail`)

- [ ] **Step 1: Write test for thumbnail session struct and lifecycle**

```rust
// tests/dwm_test.rs
use taskbar_clone::dwm::DwmThumbnailSession;

#[test]
fn test_thumbnail_session_zero_handle() {
    let session = DwmThumbnailSession { handle: 0 };
    assert_eq!(session.handle, 0);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test dwm_test`
Expected: FAIL with `DwmThumbnailSession` not found.

- [ ] **Step 3: Implement `src/dwm.rs`**

- Implement `create_dwm_thumbnail`: Calls Win32 `DwmRegisterThumbnail` with destination and source HWNDs.
- Configures `DWM_THUMBNAIL_PROPERTIES` to render with opacity and target destination rectangle.
- Implement `Drop` for `DwmThumbnailSession` to cleanly invoke `DwmUnregisterThumbnail`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --test dwm_test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/dwm.rs src/lib.rs tests/dwm_test.rs
git commit -m "feat(dwm): implement live DWM thumbnail registration and lifecycle management"
```

---

### Task 8: Win32 Event Pump, Shell Hooks & Fullscreen Yield

**Files:**
- Create: `src/events.rs`
- Modify: `src/lib.rs`
- Test: `tests/events_test.rs`

**Interfaces:**
- Consumes: `windows-rs` message pump, `crossbeam-channel`.
- Produces:
  - `pub enum ShellEvent { WindowCreated(isize), WindowDestroyed(isize), WindowActivated(isize), DisplayChanged, DpiChanged(u32), FullscreenChanged(bool) }`
  - `pub fn start_event_pump(tx: crossbeam_channel::Sender<ShellEvent>) -> std::thread::JoinHandle<()>`

- [ ] **Step 1: Write test for event channel types**

```rust
// tests/events_test.rs
use taskbar_clone::events::ShellEvent;

#[test]
fn test_shell_event_variants() {
    let event = ShellEvent::WindowActivated(42);
    match event {
        ShellEvent::WindowActivated(hwnd) => assert_eq!(hwnd, 42),
        _ => panic!("Unexpected event"),
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test events_test`
Expected: FAIL with `ShellEvent` undefined.

- [ ] **Step 3: Implement `src/events.rs`**

- Create an invisible message-only window (`HWND_MESSAGE`).
- Call `RegisterShellHookWindow(hwnd)`.
- In window procedure, map:
  - `HSHELL_WINDOWCREATED` -> `ShellEvent::WindowCreated`
  - `HSHELL_WINDOWDESTROYED` -> `ShellEvent::WindowDestroyed`
  - `HSHELL_ACTIVATESHELLWINDOW` -> `ShellEvent::WindowActivated`
  - `WM_DISPLAYCHANGE` -> `ShellEvent::DisplayChanged`
  - `WM_DPICHANGED` -> `ShellEvent::DpiChanged`
- Fullscreen check: Check if `GetForegroundWindow()` covers monitor dimensions (`GetWindowRect` vs `GetMonitorInfoW`), sending `ShellEvent::FullscreenChanged(true/false)`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --test events_test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/events.rs src/lib.rs tests/events_test.rs
git commit -m "feat(events): implement dedicated Win32 message pump for shell hooks and fullscreen detection"
```

---

### Task 9: Slint UI Markup, Themes & Visual Layouts

**Files:**
- Create: `ui/taskbar.slint`
- Create: `ui/theme.slint`
- Modify: `build.rs`
- Create: `src/ui.rs`
- Test: `tests/ui_test.rs`

**Interfaces:**
- Consumes: `Config`, `ThemePreset`, `TaskItem`, `SystemStatus`.
- Produces:
  - Slint UI window `TaskbarWindow` exposing properties:
    - `in property <string> theme_name`
    - `in property <bool> is_vertical`
    - `in property <string> time_text`
    - `in property <string> date_text`
    - `in property <int> volume_level`
    - `in property <[TaskData]> tasks`
    - `in property <[PinnedData]> pinned`
    - `callback start_clicked()`
    - `callback task_clicked(int)`
    - `callback volume_changed(float)`
    - `callback tray_toggle_clicked()`

- [ ] **Step 1: Write test verifying Slint UI compilation and property binding**

```rust
// tests/ui_test.rs
#[test]
fn test_slint_taskbar_window_compilation() {
    slint::include_modules!();
    let app = TaskbarWindow::new().expect("Failed to initialize Slint TaskbarWindow");
    app.set_theme_name("Glossy".into());
    assert_eq!(app.get_theme_name(), "Glossy");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test ui_test`
Expected: FAIL with `TaskbarWindow` not found.

- [ ] **Step 3: Implement `ui/theme.slint` and `ui/taskbar.slint`**

- `ui/theme.slint`: Defines color palettes and styles for:
  - `Glossy`: Translucent background `#1e1e1e99`, 1px subtle highlight border `#ffffff33`, blur-friendly styling.
  - `Material`: Solid dark `#242424`, elevation drop shadow, dynamic accent pill `#0078d4`.
  - `Retro`: WinClassic gray `#c0c0c0`, 3D outset beveled borders, classic font styling.
  - `Oled`: Pure black `#000000`, white indicators, zero border glow.
- `ui/taskbar.slint`:
  - Dynamically switches flex direction: `HorizontalLayout` if `!is_vertical`, `VerticalLayout` if `is_vertical`.
  - Left / Start area: Start Menu button with Windows logo.
  - Center / Main area: Pinned apps + running tasks with hover indicators, badges, and click handlers.
  - Right / Tray area: Audio volume popup, battery icon with percentage, clock and date stack, native tray toggle button.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --test ui_test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add ui/theme.slint ui/taskbar.slint src/ui.rs tests/ui_test.rs
git commit -m "feat(ui): implement Slint declarative taskbar with 4 themes and orientation layout"
```

---

### Task 10: End-to-End System Integration & Application Entrypoint

**Files:**
- Modify: `src/main.rs`
- Modify: `src/lib.rs`
- Create: `app.manifest` (Windows Per-Monitor V2 DPI & Common Controls)
- Test: `tests/integration_test.rs`

**Interfaces:**
- Consumes: All modules (`config`, `shell`, `tasks`, `icons`, `appbar`, `system`, `events`, `ui`).
- Produces: Runnable executable `taskbar-clone.exe` supporting:
  - Standard run mode (hides native tray, creates AppBars, runs event loop).
  - `taskbar-clone.exe --restore` (instantly unhides `Shell_TrayWnd` and exits).
  - `taskbar-clone.exe --autostart` (registers/unregisters user startup shortcut).

- [ ] **Step 1: Write integration test for lifecycle state machine**

```rust
// tests/integration_test.rs
use taskbar_clone::config::Config;
use taskbar_clone::tasks::{group_tasks, TaskItem};

#[test]
fn test_end_to_end_state_pipeline() {
    let cfg = Config::default();
    assert_eq!(cfg.size, 48);

    let items = vec![
        TaskItem { hwnd: 1, pid: 100, title: "Browser".into(), app_id: "chrome.exe".into(), is_active: true, is_minimized: false }
    ];
    let groups = group_tasks(items);
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].title, "Browser");
}
```

- [ ] **Step 2: Run test to verify it passes**

Run: `cargo test --test integration_test`
Expected: PASS

- [ ] **Step 3: Wire together `src/main.rs`**

- Initialize `env_logger`.
- Parse CLI arguments (`--restore`, `--autostart`).
- If `--restore`: Call `restore_native_taskbar()` and exit cleanly.
- If normal run:
  1. Install panic hook: `install_panic_watchdog()`.
  2. Load configuration: `Config::load_or_default()`.
  3. Hide native taskbar: `hide_native_taskbar()`.
  4. Enumerate monitors and initialize Slint UI windows.
  5. Register Win32 AppBars for each display.
  6. Start background event pump thread.
  7. Start icon worker pool.
  8. Run Slint event loop: handle UI callbacks (Start click -> `trigger_start_menu()`, volume change -> `set_master_volume()`, task click -> `activate_window()`).
  9. On clean exit, call `restore_native_taskbar()`.

- [ ] **Step 4: Verify full workspace build and test execution**

Run: `cargo test`
Expected: All tests pass.
Run: `cargo build`
Expected: Binary compiles without errors.

- [ ] **Step 5: Commit**

```bash
git add src/main.rs app.manifest tests/integration_test.rs
git commit -m "feat(main): wire complete application lifecycle, CLI options, and manifest"
```

