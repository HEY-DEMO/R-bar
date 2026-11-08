use taskbar_clone::appbar::{
    calculate_appbar_rect, enumerate_display_monitors, get_monitor_rect, register_appbar,
    unregister_appbar, BarRect, MonitorBar, MultiMonitorManager,
};
use taskbar_clone::config::DockEdge;

#[test]
fn test_appbar_rect_calculations_all_edges() {
    let mon = BarRect {
        left: 0,
        top: 0,
        right: 1920,
        bottom: 1080,
    };
    let size = 48;

    let bottom = calculate_appbar_rect(DockEdge::Bottom, size, mon);
    assert_eq!(
        bottom,
        BarRect {
            left: 0,
            top: 1032,
            right: 1920,
            bottom: 1080
        }
    );

    let top = calculate_appbar_rect(DockEdge::Top, size, mon);
    assert_eq!(
        top,
        BarRect {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 48
        }
    );

    let left = calculate_appbar_rect(DockEdge::Left, size, mon);
    assert_eq!(
        left,
        BarRect {
            left: 0,
            top: 0,
            right: 48,
            bottom: 1080
        }
    );

    let right = calculate_appbar_rect(DockEdge::Right, size, mon);
    assert_eq!(
        right,
        BarRect {
            left: 1872,
            top: 0,
            right: 1920,
            bottom: 1080
        }
    );
}

#[test]
fn test_appbar_rect_calculations_offset_monitor() {
    // Secondary monitor positioned to the right of primary
    let mon = BarRect {
        left: 1920,
        top: 100,
        right: 3840,
        bottom: 1180,
    };
    let size = 56;

    let bottom = calculate_appbar_rect(DockEdge::Bottom, size, mon);
    assert_eq!(
        bottom,
        BarRect {
            left: 1920,
            top: 1124,
            right: 3840,
            bottom: 1180
        }
    );

    let left = calculate_appbar_rect(DockEdge::Left, size, mon);
    assert_eq!(
        left,
        BarRect {
            left: 1920,
            top: 100,
            right: 1976,
            bottom: 1180
        }
    );
}

#[test]
fn test_monitor_enumeration() {
    let monitors = enumerate_display_monitors();
    // In any graphical Windows session there should be at least one display monitor
    assert!(!monitors.is_empty(), "Should discover at least one display monitor");

    for &hmon in &monitors {
        assert_ne!(hmon, 0, "Monitor handle should be non-zero");
        let rect_opt = get_monitor_rect(hmon);
        assert!(rect_opt.is_some(), "Monitor rect should be retrievable");
        let rect = rect_opt.unwrap();
        assert!(rect.width() > 0, "Monitor width must be positive");
        assert!(rect.height() > 0, "Monitor height must be positive");
    }
}

#[test]
fn test_monitor_bar_struct_and_methods() {
    let rect = BarRect::new(0, 1032, 1920, 1080);
    let bar = MonitorBar::new(1234, rect, 0);
    assert_eq!(bar.monitor_handle, 1234);
    assert_eq!(bar.rect, rect);
    assert_eq!(bar.hwnd, 0);
    // Unregister with 0 hwnd returns false
    assert!(!bar.unregister());
}

#[test]
fn test_invalid_monitor_handle() {
    assert_eq!(get_monitor_rect(0), None);
    assert_eq!(get_monitor_rect(-1), None);
}

#[test]
fn test_appbar_registration_invalid_hwnd() {
    assert!(!register_appbar(0, DockEdge::Bottom, 48, 0));
    assert!(!unregister_appbar(0));
    assert!(!register_appbar(999999, DockEdge::Bottom, 48, 0));
    assert!(!unregister_appbar(999999));
}

#[test]
fn test_appbar_registration_lifecycle_real_window() {
    use windows::core::{w, PCWSTR};
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DestroyWindow, WINDOW_EX_STYLE, WS_POPUP,
    };

    let monitors = enumerate_display_monitors();
    if monitors.is_empty() {
        return;
    }
    let hmon = monitors[0];

    // Create a temporary hidden test window for AppBar registration
    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("STATIC"),
            PCWSTR(std::ptr::null()),
            WS_POPUP,
            0,
            0,
            100,
            100,
            HWND(std::ptr::null_mut()),
            None,
            None,
            None,
        )
    };

    if let Ok(hwnd) = hwnd {
        let hwnd_val = hwnd.0 as isize;
        let reg_ok = register_appbar(hwnd_val, DockEdge::Bottom, 48, hmon);
        assert!(reg_ok, "register_appbar should succeed with valid window");

        let unreg_ok = unregister_appbar(hwnd_val);
        assert!(unreg_ok, "unregister_appbar should succeed with valid window");

        unsafe {
            let _ = DestroyWindow(hwnd);
        }
    }
}

#[test]
fn test_multimonitor_manager_lifecycle() {
    let mut manager = MultiMonitorManager::new(DockEdge::Bottom, 48);
    assert_eq!(manager.edge(), DockEdge::Bottom);
    assert_eq!(manager.size(), 48);

    let spawned = manager.spawn_bars();
    assert!(spawned > 0, "Should spawn at least one bar for connected displays");
    assert_eq!(manager.bars().len(), spawned);

    for bar in manager.bars() {
        assert_ne!(bar.monitor_handle, 0);
        assert_ne!(bar.hwnd, 0);
        assert!(bar.rect.width() > 0);
        assert_eq!(bar.rect.height(), 48);
    }

    // Unregister all
    manager.unregister_all();
    assert!(manager.bars().is_empty());
}

#[test]
fn test_multimonitor_manager_dpi_change() {
    let mut manager = MultiMonitorManager::new(DockEdge::Bottom, 48);
    manager.spawn_bars();

    // 144 DPI corresponds to 150% scaling (48 * 1.5 = 72)
    manager.handle_dpi_change(144);
    for bar in manager.bars() {
        assert_eq!(bar.rect.height(), 72);
    }

    // Reset back to standard 96 DPI (100% -> 48)
    manager.handle_dpi_change(96);
    for bar in manager.bars() {
        assert_eq!(bar.rect.height(), 48);
    }

    manager.unregister_all();
}

#[test]
fn test_multimonitor_manager_display_change() {
    let mut manager = MultiMonitorManager::new(DockEdge::Top, 40);
    manager.spawn_bars();
    assert!(!manager.bars().is_empty());

    // Calling handle_display_change should refresh / reconcile connected monitors
    manager.handle_display_change();
    assert!(!manager.bars().is_empty());
    for bar in manager.bars() {
        assert_eq!(bar.rect.height(), 40);
    }

    manager.unregister_all();
}
