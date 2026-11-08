use crossbeam_channel::Sender;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{
    HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM,
};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DeregisterShellHookWindow, DestroyWindow, DispatchMessageW,
    GetDesktopWindow, GetMessageW, GetShellWindow, GetWindowLongPtrW, GetWindowRect,
    PostQuitMessage, RegisterClassExW, RegisterShellHookWindow, RegisterWindowMessageW,
    SetWindowLongPtrW, TranslateMessage, CREATESTRUCTW, GWLP_USERDATA, MSG, WINDOW_EX_STYLE,
    WNDCLASSEXW, WS_POPUP, WM_CLOSE, WM_CREATE, WM_DESTROY, WM_DISPLAYCHANGE, WM_DPICHANGED,
};

pub use crate::autohide::is_rect_fullscreen;

pub const HSHELL_WINDOWCREATED: u32 = 1;
pub const HSHELL_WINDOWDESTROYED: u32 = 2;
pub const HSHELL_ACTIVATESHELLWINDOW: u32 = 4;
pub const HSHELL_WINDOWACTIVATED: u32 = 4;
pub const HSHELL_GETMINRECT: u32 = 5;
pub const HSHELL_REDRAW: u32 = 6;
pub const HSHELL_TASKMAN: u32 = 7;
pub const HSHELL_LANGUAGE: u32 = 8;
pub const HSHELL_SYSMENU: u32 = 9;
pub const HSHELL_ENDTASK: u32 = 10;
pub const HSHELL_ACCESSIBILITYSTATE: u32 = 11;
pub const HSHELL_APPCOMMAND: u32 = 12;
pub const HSHELL_WINDOWREPLACED: u32 = 13;
pub const HSHELL_WINDOWREPLACING: u32 = 14;
pub const HSHELL_HIGHBIT: u32 = 0x8000;
pub const HSHELL_FLASH: u32 = HSHELL_REDRAW | HSHELL_HIGHBIT;
pub const HSHELL_RUDEAPPACTIVATED: u32 = HSHELL_ACTIVATESHELLWINDOW | HSHELL_HIGHBIT;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellEvent {
    WindowCreated(isize),
    WindowDestroyed(isize),
    WindowActivated(isize),
    WindowRedraw(isize),
    DisplayChanged,
    DpiChanged(u32),
    FullscreenChanged(bool),
}

pub fn parse_shell_hook_event(code: usize, hwnd: isize) -> Option<ShellEvent> {
    let code_u32 = code as u32;
    let base_code = code_u32 & !HSHELL_HIGHBIT;
    match base_code {
        HSHELL_WINDOWCREATED => Some(ShellEvent::WindowCreated(hwnd)),
        HSHELL_WINDOWDESTROYED => Some(ShellEvent::WindowDestroyed(hwnd)),
        HSHELL_ACTIVATESHELLWINDOW => Some(ShellEvent::WindowActivated(hwnd)),
        HSHELL_REDRAW => Some(ShellEvent::WindowRedraw(hwnd)),
        _ => None,
    }
}

pub fn is_window_fullscreen(hwnd: HWND) -> bool {
    if hwnd.0.is_null() {
        return false;
    }
    unsafe {
        if hwnd == GetDesktopWindow() || hwnd == GetShellWindow() {
            return false;
        }
    }
    let mut rect = RECT::default();
    if unsafe { GetWindowRect(hwnd, &mut rect) }.is_err() {
        return false;
    }
    let hmonitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
    let mut minfo = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    if unsafe { GetMonitorInfoW(hmonitor, &mut minfo) }.as_bool() {
        crate::autohide::is_rect_fullscreen(
            crate::geometry::BarRect::from(rect),
            crate::geometry::BarRect::from(minfo.rcMonitor),
        )
    } else {
        false
    }
}

struct EventPumpContext {
    tx: Sender<ShellEvent>,
    shell_hook_msg: u32,
    was_fullscreen: bool,
    running: Arc<AtomicBool>,
}

unsafe extern "system" fn event_pump_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_CREATE {
        let cs = unsafe { &*(lparam.0 as *const CREATESTRUCTW) };
        unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, cs.lpCreateParams as isize) };
        return LRESULT(0);
    }

    let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut EventPumpContext };
    if !ptr.is_null() {
        let ctx = unsafe { &mut *ptr };

        if msg == ctx.shell_hook_msg && ctx.shell_hook_msg != 0 {
            let code = wparam.0;
            let target_hwnd = lparam.0;

            if let Some(event) = parse_shell_hook_event(code, target_hwnd) {
                // If window activated, perform fullscreen detection
                if let ShellEvent::WindowActivated(act_hwnd) = event {
                    let is_fs = is_window_fullscreen(HWND(act_hwnd as *mut _));
                    if is_fs != ctx.was_fullscreen {
                        ctx.was_fullscreen = is_fs;
                        let _ = ctx.tx.send(ShellEvent::FullscreenChanged(is_fs));
                    }
                }

                if ctx.tx.send(event).is_err() {
                    // Receiver dropped, exit pump
                    ctx.running.store(false, Ordering::SeqCst);
                    let _ = unsafe { DestroyWindow(hwnd) };
                }
            }
            return LRESULT(0);
        }

        if msg == WM_DISPLAYCHANGE {
            let _ = ctx.tx.send(ShellEvent::DisplayChanged);
            return LRESULT(0);
        }

        if msg == WM_DPICHANGED {
            let dpi = (wparam.0 & 0xFFFF) as u32;
            let _ = ctx.tx.send(ShellEvent::DpiChanged(dpi));
            return LRESULT(0);
        }

        if msg == WM_CLOSE {
            let _ = unsafe { DestroyWindow(hwnd) };
            return LRESULT(0);
        }

        if msg == WM_DESTROY {
            let _ = unsafe { DeregisterShellHookWindow(hwnd) };
            ctx.running.store(false, Ordering::SeqCst);
            unsafe { PostQuitMessage(0) };
            return LRESULT(0);
        }
    }

    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

pub fn start_event_pump(tx: Sender<ShellEvent>) -> std::thread::JoinHandle<()> {
    let running = Arc::new(AtomicBool::new(true));
    let running_thread = running.clone();

    std::thread::spawn(move || {
        let class_name = w!("TaskbarClone_ShellHookEventWindow");
        let hinstance = HINSTANCE(std::ptr::null_mut());

        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(event_pump_wndproc),
            hInstance: hinstance,
            lpszClassName: class_name,
            ..Default::default()
        };

        unsafe { RegisterClassExW(&wc) };

        let shell_hook_msg = unsafe { RegisterWindowMessageW(w!("SHELLHOOK")) };

        let mut context = Box::new(EventPumpContext {
            tx,
            shell_hook_msg,
            was_fullscreen: false,
            running: running_thread.clone(),
        });

        let hwnd = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                class_name,
                PCWSTR(std::ptr::null()),
                WS_POPUP,
                0,
                0,
                0,
                0,
                HWND(std::ptr::null_mut()),
                None,
                hinstance,
                Some(context.as_mut() as *mut _ as *const std::ffi::c_void),
            )
        };

        if let Ok(hwnd) = hwnd {
            let _ = unsafe { RegisterShellHookWindow(hwnd) };

            let mut msg = MSG::default();
            while running_thread.load(Ordering::SeqCst)
                && unsafe { GetMessageW(&mut msg, None, 0, 0).as_bool() }
            {
                unsafe {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }

            let _ = unsafe { DeregisterShellHookWindow(hwnd) };
            let _ = unsafe { DestroyWindow(hwnd) };
        }
    })
}

pub fn stop_event_pump() {
    let hwnd = unsafe {
        windows::Win32::UI::WindowsAndMessaging::FindWindowW(
            w!("TaskbarClone_ShellHookEventWindow"),
            PCWSTR(std::ptr::null()),
        )
    };
    if let Ok(h) = hwnd {
        if !h.0.is_null() {
            let _ = unsafe {
                windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                    h,
                    WM_CLOSE,
                    WPARAM(0),
                    LPARAM(0),
                )
            };
        }
    }
}

