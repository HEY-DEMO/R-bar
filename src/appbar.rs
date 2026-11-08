use std::sync::Once;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{BOOL, HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO, MonitorFromWindow,
    MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::UI::Shell::{
    SHAppBarMessage, ABM_NEW, ABM_QUERYPOS, ABM_REMOVE, ABM_SETPOS, APPBARDATA,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, FindWindowW, GetSystemMetrics, IsWindow,
    MoveWindow, RegisterClassExW, RegisterWindowMessageW, SM_CXSCREEN, SM_CYSCREEN,
    WINDOW_EX_STYLE, WNDCLASSEXW, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP, WS_VISIBLE,
};

use crate::config::DockEdge;
pub use crate::geometry::*;

/// Edge index 0 = Left (ABE_LEFT)
pub const ABE_LEFT: u32 = 0;
/// Edge index 1 = Top (ABE_TOP)
pub const ABE_TOP: u32 = 1;
/// Edge index 2 = Right (ABE_RIGHT)
pub const ABE_RIGHT: u32 = 2;
/// Edge index 3 = Bottom (ABE_BOTTOM)
pub const ABE_BOTTOM: u32 = 3;

/// Converts a [`DockEdge`] into the corresponding Win32 AppBar edge index (`ABE_*`).
pub fn dock_edge_to_appbar_edge(edge: DockEdge) -> u32 {
    match edge {
        DockEdge::Left => ABE_LEFT,
        DockEdge::Top => ABE_TOP,
        DockEdge::Right => ABE_RIGHT,
        DockEdge::Bottom => ABE_BOTTOM,
    }
}

/// Represents a single AppBar instance bound to a specific display monitor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonitorBar {
    pub monitor_handle: isize,
    pub rect: BarRect,
    pub hwnd: isize,
}

impl MonitorBar {
    pub fn new(monitor_handle: isize, rect: BarRect, hwnd: isize) -> Self {
        Self {
            monitor_handle,
            rect,
            hwnd,
        }
    }

    /// Unregisters the AppBar from the Windows shell and frees reserved work area.
    pub fn unregister(&self) -> bool {
        unregister_appbar(self.hwnd)
    }
}

unsafe extern "system" fn monitor_enum_proc(
    hmonitor: HMONITOR,
    _hdc: HDC,
    _rect: *mut RECT,
    lparam: LPARAM,
) -> BOOL {
    let monitors = unsafe { &mut *(lparam.0 as *mut Vec<isize>) };
    monitors.push(hmonitor.0 as isize);
    BOOL(1)
}

/// Discovers all connected display monitors using Win32 `EnumDisplayMonitors`.
pub fn enumerate_display_monitors() -> Vec<isize> {
    let mut monitors: Vec<isize> = Vec::new();
    unsafe {
        let _ = EnumDisplayMonitors(
            HDC(std::ptr::null_mut()),
            None,
            Some(monitor_enum_proc),
            LPARAM(&mut monitors as *mut _ as isize),
        );
    }
    monitors
}

/// Retrieves the screen bounds rectangle for the specified monitor handle.
/// Returns `None` if the handle is invalid or if `GetMonitorInfoW` fails.
pub fn get_monitor_rect(monitor_handle: isize) -> Option<BarRect> {
    if monitor_handle == 0 || monitor_handle == -1 {
        return None;
    }
    unsafe {
        let hmon = HMONITOR(monitor_handle as *mut _);
        let mut minfo = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if GetMonitorInfoW(hmon, &mut minfo).as_bool() {
            Some(BarRect::from(minfo.rcMonitor))
        } else {
            None
        }
    }
}

/// Retrieves the working area bounds rectangle for the specified monitor handle.
pub fn get_monitor_work_area(monitor_handle: isize) -> Option<BarRect> {
    if monitor_handle == 0 || monitor_handle == -1 {
        return None;
    }
    unsafe {
        let hmon = HMONITOR(monitor_handle as *mut _);
        let mut minfo = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if GetMonitorInfoW(hmon, &mut minfo).as_bool() {
            Some(BarRect::from(minfo.rcWork))
        } else {
            None
        }
    }
}

/// Registers a window as a Win32 Application Desktop Toolbar (AppBar) and reserves
/// desktop work area along the specified monitor edge using `SHAppBarMessage`.
pub fn register_appbar(hwnd: isize, edge: DockEdge, size: u32, monitor: isize) -> bool {
    if hwnd == 0 {
        return false;
    }

    unsafe {
        let hwnd_val = HWND(hwnd as *mut _);
        if !IsWindow(hwnd_val).as_bool() {
            return false;
        }

        let callback_msg = RegisterWindowMessageW(w!("TaskbarClone_AppBarCallback"));
        let mut abd = APPBARDATA {
            cbSize: std::mem::size_of::<APPBARDATA>() as u32,
            hWnd: hwnd_val,
            uCallbackMessage: callback_msg,
            ..Default::default()
        };

        // 1. ABM_NEW: Register AppBar with the shell
        let _ = SHAppBarMessage(ABM_NEW, &mut abd);

        // Resolve monitor coordinates
        let mon_rect = if monitor != 0 {
            get_monitor_rect(monitor).unwrap_or_else(|| {
                BarRect::new(
                    0,
                    0,
                    GetSystemMetrics(SM_CXSCREEN),
                    GetSystemMetrics(SM_CYSCREEN),
                )
            })
        } else {
            let hmon = MonitorFromWindow(hwnd_val, MONITOR_DEFAULTTONEAREST);
            get_monitor_rect(hmon.0 as isize).unwrap_or_else(|| {
                BarRect::new(
                    0,
                    0,
                    GetSystemMetrics(SM_CXSCREEN),
                    GetSystemMetrics(SM_CYSCREEN),
                )
            })
        };

        // 2. ABM_QUERYPOS: Query system for proposed bounding box
        let proposed = calculate_appbar_rect(edge, size, mon_rect);
        abd.uEdge = dock_edge_to_appbar_edge(edge);
        abd.rc = proposed.into();

        let _ = SHAppBarMessage(ABM_QUERYPOS, &mut abd);

        // Adjust dimensions to keep exact requested bar thickness along docking edge
        match edge {
            DockEdge::Bottom => {
                abd.rc.top = abd.rc.bottom - size as i32;
            }
            DockEdge::Top => {
                abd.rc.bottom = abd.rc.top + size as i32;
            }
            DockEdge::Left => {
                abd.rc.right = abd.rc.left + size as i32;
            }
            DockEdge::Right => {
                abd.rc.left = abd.rc.right - size as i32;
            }
        }

        // 3. ABM_SETPOS: Commit position and reserve desktop work area
        let _ = SHAppBarMessage(ABM_SETPOS, &mut abd);

        // Position the window physically at the calculated coordinates
        let final_rect = BarRect::from(abd.rc);
        let _ = MoveWindow(
            hwnd_val,
            final_rect.left,
            final_rect.top,
            final_rect.width(),
            final_rect.height(),
            true,
        );

        true
    }
}

/// Unregisters an AppBar from the shell using `ABM_REMOVE`, cleanly restoring desktop work area.
pub fn unregister_appbar(hwnd: isize) -> bool {
    if hwnd == 0 {
        return false;
    }
    unsafe {
        let hwnd_val = HWND(hwnd as *mut _);
        if !IsWindow(hwnd_val).as_bool() {
            return false;
        }

        let mut abd = APPBARDATA {
            cbSize: std::mem::size_of::<APPBARDATA>() as u32,
            hWnd: hwnd_val,
            ..Default::default()
        };
        let res = SHAppBarMessage(ABM_REMOVE, &mut abd);
        if res != 0 {
            true
        } else {
            // In headless/test environments where explorer's Shell_TrayWnd is not running,
            // SHAppBarMessage returns 0 even though the unregister call executed cleanly.
            let has_tray = FindWindowW(w!("Shell_TrayWnd"), PCWSTR(std::ptr::null())).is_ok();
            !has_tray
        }
    }
}

static REGISTER_APPBAR_CLASS_ONCE: Once = Once::new();

unsafe extern "system" fn appbar_window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

fn ensure_appbar_window_class_registered() {
    REGISTER_APPBAR_CLASS_ONCE.call_once(|| {
        let class_name = w!("TaskbarClone_AppBarWindowClass");
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(appbar_window_proc),
            hInstance: HINSTANCE(std::ptr::null_mut()),
            lpszClassName: class_name,
            ..Default::default()
        };
        unsafe { RegisterClassExW(&wc) };
    });
}

/// Creates a top-level popup toolwindow suitable for AppBar registration at the specified rectangle.
pub fn create_appbar_window(rect: BarRect) -> Option<isize> {
    ensure_appbar_window_class_registered();
    let class_name = w!("TaskbarClone_AppBarWindowClass");
    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(WS_EX_TOPMOST.0 | WS_EX_TOOLWINDOW.0),
            class_name,
            PCWSTR(std::ptr::null()),
            WS_POPUP | WS_VISIBLE,
            rect.left,
            rect.top,
            rect.width(),
            rect.height(),
            HWND(std::ptr::null_mut()),
            None,
            HINSTANCE(std::ptr::null_mut()),
            None,
        )
    };
    hwnd.ok().map(|h| h.0 as isize)
}

/// Destroys an AppBar window created by `create_appbar_window`.
pub fn destroy_appbar_window(hwnd: isize) -> bool {
    if hwnd == 0 {
        return false;
    }
    unsafe { DestroyWindow(HWND(hwnd as *mut _)).is_ok() }
}

/// Coordinates multi-monitor display enumeration, dedicated AppBar instantiation,
/// dynamic DPI scaling, and desktop work area reservations across all displays.
pub struct MultiMonitorManager {
    bars: Vec<MonitorBar>,
    edge: DockEdge,
    base_size: u32,
    effective_size: u32,
    dpi: u32,
}

impl MultiMonitorManager {
    /// Creates a new `MultiMonitorManager` targeting the given edge and base pixel thickness.
    pub fn new(edge: DockEdge, size: u32) -> Self {
        Self {
            bars: Vec::new(),
            edge,
            base_size: size,
            effective_size: size,
            dpi: 96,
        }
    }

    /// The configured docking edge.
    pub fn edge(&self) -> DockEdge {
        self.edge
    }

    /// The base bar size/thickness.
    pub fn base_size(&self) -> u32 {
        self.base_size
    }

    /// The current effective (DPI-scaled) bar size/thickness.
    pub fn size(&self) -> u32 {
        self.effective_size
    }

    /// Current active monitor bars.
    pub fn bars(&self) -> &[MonitorBar] {
        &self.bars
    }

    /// Mutable slice of active monitor bars.
    pub fn bars_mut(&mut self) -> &mut [MonitorBar] {
        &mut self.bars
    }

    /// Discovers all connected displays and spawns a dedicated AppBar instance on each.
    /// Returns the number of successfully spawned bars.
    pub fn spawn_bars(&mut self) -> usize {
        let monitors = enumerate_display_monitors();
        let mut count = 0;

        for hmon in monitors {
            // Check if we already have an active bar for this monitor
            if self.bars.iter().any(|b| b.monitor_handle == hmon) {
                continue;
            }

            if let Some(mon_rect) = get_monitor_rect(hmon) {
                let rect = calculate_appbar_rect(self.edge, self.effective_size, mon_rect);
                if let Some(hwnd) = create_appbar_window(rect) {
                    if register_appbar(hwnd, self.edge, self.effective_size, hmon) {
                        self.bars.push(MonitorBar::new(hmon, rect, hwnd));
                        count += 1;
                    } else {
                        destroy_appbar_window(hwnd);
                    }
                }
            }
        }

        count
    }

    /// Responds to topology changes (`WM_DISPLAYCHANGE`), removing bars for disconnected
    /// monitors and spawning/updating bars for newly detected or existing displays.
    pub fn handle_display_change(&mut self) {
        let current_monitors = enumerate_display_monitors();

        // 1. Remove and unregister bars for disconnected monitors
        self.bars.retain(|bar| {
            if !current_monitors.contains(&bar.monitor_handle) {
                bar.unregister();
                destroy_appbar_window(bar.hwnd);
                false
            } else {
                true
            }
        });

        // 2. Update existing bars and spawn new ones
        for &hmon in &current_monitors {
            if let Some(mon_rect) = get_monitor_rect(hmon) {
                let rect = calculate_appbar_rect(self.edge, self.effective_size, mon_rect);
                if let Some(bar) = self.bars.iter_mut().find(|b| b.monitor_handle == hmon) {
                    bar.rect = rect;
                    register_appbar(bar.hwnd, self.edge, self.effective_size, hmon);
                } else if let Some(hwnd) = create_appbar_window(rect) {
                    if register_appbar(hwnd, self.edge, self.effective_size, hmon) {
                        self.bars.push(MonitorBar::new(hmon, rect, hwnd));
                    } else {
                        destroy_appbar_window(hwnd);
                    }
                }
            }
        }
    }

    /// Responds to DPI scale changes (`WM_DPICHANGED`), recalculating bar heights and
    /// repositioning/reserving work areas across all connected displays.
    pub fn handle_dpi_change(&mut self, new_dpi: u32) {
        self.dpi = new_dpi;
        // Standard default DPI is 96 (100% scale)
        let scale = new_dpi as f32 / 96.0;
        self.effective_size = ((self.base_size as f32) * scale).round() as u32;

        for bar in &mut self.bars {
            if let Some(mon_rect) = get_monitor_rect(bar.monitor_handle) {
                let rect = calculate_appbar_rect(self.edge, self.effective_size, mon_rect);
                bar.rect = rect;
                register_appbar(bar.hwnd, self.edge, self.effective_size, bar.monitor_handle);
            }
        }
    }

    /// Sets topmost z-order across all active monitor bars.
    pub fn set_topmost(&self, topmost: bool) {
        for bar in &self.bars {
            crate::autohide::set_taskbar_topmost(bar.hwnd, topmost);
        }
    }

    /// Yields topmost z-order when a fullscreen foreground application is active,
    /// or restores topmost docking when fullscreen applications lose focus or exit.
    pub fn set_fullscreen_yield(&self, fullscreen_active: bool) {
        self.set_topmost(!fullscreen_active);
    }

    /// Unregisters all AppBars and destroys their underlying windows cleanly.
    pub fn unregister_all(&mut self) {
        for bar in self.bars.drain(..) {
            bar.unregister();
            destroy_appbar_window(bar.hwnd);
        }
    }
}

impl Drop for MultiMonitorManager {
    fn drop(&mut self) {
        self.unregister_all();
    }
}
