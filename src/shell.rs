use windows::core::w;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    keybd_event, KEYEVENTF_KEYUP, VK_A, VK_LWIN,
};
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowExW, FindWindowW, ShowWindow, SW_HIDE, SW_SHOW,
};

/// Returns the class names of native Windows taskbars.
pub fn get_native_taskbar_classes() -> Vec<&'static str> {
    vec!["Shell_TrayWnd", "Shell_SecondaryTrayWnd"]
}

/// Finds all native taskbar window handles across primary and secondary displays.
pub fn find_native_taskbars() -> Vec<HWND> {
    let mut hwnds = Vec::new();
    unsafe {
        // Find primary taskbar: Shell_TrayWnd
        if let Ok(hwnd) = FindWindowW(w!("Shell_TrayWnd"), None) {
            if !hwnd.0.is_null() {
                hwnds.push(hwnd);
            }
        }

        // Find all secondary taskbars: Shell_SecondaryTrayWnd
        let mut prev = HWND(std::ptr::null_mut());
        loop {
            match FindWindowExW(HWND(std::ptr::null_mut()), prev, w!("Shell_SecondaryTrayWnd"), None) {
                Ok(hwnd) if !hwnd.0.is_null() => {
                    hwnds.push(hwnd);
                    prev = hwnd;
                }
                _ => break,
            }
        }
    }
    hwnds
}

/// Conceals the native Windows 11 taskbar window and secondary monitor trays.
pub fn hide_native_taskbar() {
    let hwnds = find_native_taskbars();
    for hwnd in hwnds {
        unsafe {
            let _ = ShowWindow(hwnd, SW_HIDE);
        }
    }
}

/// Unhides and restores the native Windows 11 taskbar and secondary monitor trays.
pub fn restore_native_taskbar() {
    let hwnds = find_native_taskbars();
    for hwnd in hwnds {
        unsafe {
            let _ = ShowWindow(hwnd, SW_SHOW);
        }
    }
}

/// Installs a panic watchdog hook so any unexpected process panic immediately restores the native taskbar.
pub fn install_panic_watchdog() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        log::error!("Taskbar clone panicked! Restoring native taskbar: {:?}", panic_info);
        eprintln!("Taskbar clone panicked! Restoring native Windows taskbar...");
        restore_native_taskbar();
        default_hook(panic_info);
    }));
}

/// Triggers the native Windows Start Menu by synthesizing a Windows key press.
pub fn trigger_start_menu() {
    unsafe {
        keybd_event(VK_LWIN.0 as u8, 0, Default::default(), 0);
        keybd_event(VK_LWIN.0 as u8, 0, KEYEVENTF_KEYUP, 0);
    }
}

/// Triggers the native Windows Quick Settings flyout (Win + A).
pub fn trigger_quick_settings() {
    unsafe {
        keybd_event(VK_LWIN.0 as u8, 0, Default::default(), 0);
        keybd_event(VK_A.0 as u8, 0, Default::default(), 0);
        keybd_event(VK_A.0 as u8, 0, KEYEVENTF_KEYUP, 0);
        keybd_event(VK_LWIN.0 as u8, 0, KEYEVENTF_KEYUP, 0);
    }
}

