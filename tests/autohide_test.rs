use taskbar_clone::appbar::{create_appbar_window, destroy_appbar_window, MultiMonitorManager};
use taskbar_clone::autohide::{
    calculate_hidden_rect, is_cursor_at_edge, is_cursor_in_rect, is_rect_fullscreen,
    set_taskbar_topmost, set_window_topmost, AutoHideController, AutoHideState,
    DEFAULT_AUTO_HIDE_PEEK, DEFAULT_EDGE_TRIGGER_THRESHOLD,
};
use taskbar_clone::config::DockEdge;
use taskbar_clone::events::{is_window_fullscreen, ShellEvent};
use taskbar_clone::geometry::BarRect;
use taskbar_clone::system::{
    find_native_tray_window, find_tray_notify_window, is_native_tray_visible, toggle_native_tray,
};
use windows::Win32::Foundation::HWND;

// ============================================================================
// 1. Fullscreen Detection Tests
// ============================================================================

#[test]
fn test_fullscreen_rect_detection_exact_match() {
    let monitor = BarRect::new(0, 0, 1920, 1080);
    let fullscreen_window = BarRect::new(0, 0, 1920, 1080);

    assert!(
        is_rect_fullscreen(fullscreen_window, monitor),
        "Exact monitor bounds match must be detected as fullscreen"
    );
}

#[test]
fn test_fullscreen_rect_detection_borderless_oversized() {
    let monitor = BarRect::new(0, 0, 1920, 1080);
    // Borderless fullscreen games and media players often extend 1-3px past screen boundaries
    let borderless_game = BarRect::new(-2, -2, 1922, 1082);

    assert!(
        is_rect_fullscreen(borderless_game, monitor),
        "Oversized borderless game window covering display must be detected as fullscreen"
    );
}

#[test]
fn test_fullscreen_rect_detection_maximized_not_fullscreen() {
    let monitor = BarRect::new(0, 0, 1920, 1080);
    // Normal maximized window respects the AppBar work area (bottom edge at 1032 for 48px bar)
    let maximized_app = BarRect::new(0, 0, 1920, 1032);

    assert!(
        !is_rect_fullscreen(maximized_app, monitor),
        "Standard maximized window respecting work area must NOT be detected as fullscreen"
    );
}

#[test]
fn test_fullscreen_rect_detection_floating_and_minimized() {
    let monitor = BarRect::new(0, 0, 1920, 1080);
    let floating_window = BarRect::new(100, 100, 900, 700);
    let minimized_window = BarRect::new(0, 0, 0, 0);

    assert!(!is_rect_fullscreen(floating_window, monitor));
    assert!(!is_rect_fullscreen(minimized_window, monitor));
}

#[test]
fn test_fullscreen_rect_detection_secondary_offset_monitor() {
    // Secondary monitor positioned at negative coordinates
    let sec_monitor = BarRect::new(-1920, -120, 0, 960);
    let sec_fs_window = BarRect::new(-1922, -122, 2, 962);
    let primary_window = BarRect::new(0, 0, 1920, 1080);

    assert!(
        is_rect_fullscreen(sec_fs_window, sec_monitor),
        "Fullscreen window on offset secondary monitor must be detected"
    );
    assert!(
        !is_rect_fullscreen(primary_window, sec_monitor),
        "Primary window coordinates must not register as fullscreen on secondary monitor"
    );
}

#[test]
fn test_live_window_fullscreen_null_and_desktop_safety() {
    // Null handle should never be considered fullscreen
    assert!(!is_window_fullscreen(HWND(std::ptr::null_mut())));

    // Desktop window should never be classified as fullscreen game
    unsafe {
        let desktop = windows::Win32::UI::WindowsAndMessaging::GetDesktopWindow();
        assert!(
            !is_window_fullscreen(desktop),
            "Desktop window must not trigger fullscreen yield"
        );
    }
}

// ============================================================================
// 2. Fullscreen Z-Order Yield & Restoration Tests
// ============================================================================

#[test]
fn test_taskbar_topmost_z_order_toggle() {
    // Null / invalid handle returns false
    assert!(!set_taskbar_topmost(0, true));
    assert!(!set_taskbar_topmost(0, false));
    assert!(!set_window_topmost(0, true));

    // Create a real test window and verify topmost toggling
    let rect = BarRect::new(0, 1032, 1920, 1080);
    if let Some(hwnd) = create_appbar_window(rect) {
        // Yield z-order: set to HWND_NOTOPMOST
        let yield_res = set_taskbar_topmost(hwnd, false);
        assert!(yield_res, "Yielding topmost z-order must succeed");

        // Re-engage topmost: set to HWND_TOPMOST
        let restore_res = set_taskbar_topmost(hwnd, true);
        assert!(restore_res, "Restoring topmost z-order must succeed");

        destroy_appbar_window(hwnd);
    }
}

#[test]
fn test_shell_event_fullscreen_changed_variant() {
    let enter_fs = ShellEvent::FullscreenChanged(true);
    let exit_fs = ShellEvent::FullscreenChanged(false);

    match enter_fs {
        ShellEvent::FullscreenChanged(true) => {}
        _ => panic!("Expected FullscreenChanged(true)"),
    }

    match exit_fs {
        ShellEvent::FullscreenChanged(false) => {}
        _ => panic!("Expected FullscreenChanged(false)"),
    }
}

#[test]
fn test_multi_monitor_manager_fullscreen_yield() {
    let mut manager = MultiMonitorManager::new(DockEdge::Bottom, 48);
    let spawned = manager.spawn_bars();

    if spawned > 0 {
        // Test yielding z-order across all bars
        manager.set_fullscreen_yield(true);
        // Test restoring topmost across all bars
        manager.set_fullscreen_yield(false);
    }

    manager.unregister_all();
}

// ============================================================================
// 3. Auto-Hide Geometry Calculations Across All 4 Edges
// ============================================================================

#[test]
fn test_calculate_hidden_rect_all_four_edges_with_peek() {
    let monitor = BarRect::new(0, 0, 1920, 1080);
    let size = 48;
    let peek = 2;

    // 1. Bottom Edge
    let bottom_vis = taskbar_clone::geometry::calculate_appbar_rect(DockEdge::Bottom, size, monitor);
    let bottom_hidden = calculate_hidden_rect(DockEdge::Bottom, bottom_vis, peek);
    assert_eq!(bottom_hidden.width(), 1920);
    assert_eq!(bottom_hidden.height(), 48);
    assert_eq!(bottom_hidden.top, 1080 - peek);
    assert_eq!(bottom_hidden.bottom, 1080 - peek + 48);

    // 2. Top Edge
    let top_vis = taskbar_clone::geometry::calculate_appbar_rect(DockEdge::Top, size, monitor);
    let top_hidden = calculate_hidden_rect(DockEdge::Top, top_vis, peek);
    assert_eq!(top_hidden.width(), 1920);
    assert_eq!(top_hidden.height(), 48);
    assert_eq!(top_hidden.bottom, top_vis.top + peek);
    assert_eq!(top_hidden.top, top_vis.top + peek - 48);

    // 3. Left Edge
    let left_vis = taskbar_clone::geometry::calculate_appbar_rect(DockEdge::Left, size, monitor);
    let left_hidden = calculate_hidden_rect(DockEdge::Left, left_vis, peek);
    assert_eq!(left_hidden.width(), 48);
    assert_eq!(left_hidden.height(), 1080);
    assert_eq!(left_hidden.right, left_vis.left + peek);
    assert_eq!(left_hidden.left, left_vis.left + peek - 48);

    // 4. Right Edge
    let right_vis = taskbar_clone::geometry::calculate_appbar_rect(DockEdge::Right, size, monitor);
    let right_hidden = calculate_hidden_rect(DockEdge::Right, right_vis, peek);
    assert_eq!(right_hidden.width(), 48);
    assert_eq!(right_hidden.height(), 1080);
    assert_eq!(right_hidden.left, 1920 - peek);
    assert_eq!(right_hidden.right, 1920 - peek + 48);
}

#[test]
fn test_calculate_hidden_rect_zero_peek_completely_offscreen() {
    let monitor = BarRect::new(0, 0, 1920, 1080);
    let size = 48;
    let peek = 0;

    let bottom_vis = taskbar_clone::geometry::calculate_appbar_rect(DockEdge::Bottom, size, monitor);
    let bottom_hidden = calculate_hidden_rect(DockEdge::Bottom, bottom_vis, peek);
    assert_eq!(bottom_hidden.top, 1080);
    assert_eq!(bottom_hidden.bottom, 1128);

    let top_vis = taskbar_clone::geometry::calculate_appbar_rect(DockEdge::Top, size, monitor);
    let top_hidden = calculate_hidden_rect(DockEdge::Top, top_vis, peek);
    assert_eq!(top_hidden.bottom, 0);
    assert_eq!(top_hidden.top, -48);
}

#[test]
fn test_calculate_hidden_rect_offset_secondary_monitor() {
    let sec_monitor = BarRect::new(-1920, -120, 0, 960);
    let size = 56;
    let peek = 3;

    let bottom_vis = taskbar_clone::geometry::calculate_appbar_rect(DockEdge::Bottom, size, sec_monitor);
    let bottom_hidden = calculate_hidden_rect(DockEdge::Bottom, bottom_vis, peek);
    assert_eq!(bottom_hidden.left, -1920);
    assert_eq!(bottom_hidden.right, 0);
    assert_eq!(bottom_hidden.top, 960 - peek);
    assert_eq!(bottom_hidden.bottom, 960 - peek + 56);
}

// ============================================================================
// 4. Hover Trigger Zone & Edge Cursor Detection Tests
// ============================================================================

#[test]
fn test_is_cursor_at_edge_all_four_edges() {
    let monitor = BarRect::new(0, 0, 1920, 1080);
    let threshold = 4;

    // Bottom edge: y near 1080
    assert!(is_cursor_at_edge(960, 1079, DockEdge::Bottom, monitor, threshold));
    assert!(is_cursor_at_edge(960, 1080, DockEdge::Bottom, monitor, threshold));
    assert!(is_cursor_at_edge(960, 1076, DockEdge::Bottom, monitor, threshold));
    assert!(!is_cursor_at_edge(960, 1070, DockEdge::Bottom, monitor, threshold));
    assert!(!is_cursor_at_edge(-10, 1079, DockEdge::Bottom, monitor, threshold)); // outside X

    // Top edge: y near 0
    assert!(is_cursor_at_edge(960, 0, DockEdge::Top, monitor, threshold));
    assert!(is_cursor_at_edge(960, 3, DockEdge::Top, monitor, threshold));
    assert!(!is_cursor_at_edge(960, 10, DockEdge::Top, monitor, threshold));

    // Left edge: x near 0
    assert!(is_cursor_at_edge(0, 540, DockEdge::Left, monitor, threshold));
    assert!(is_cursor_at_edge(3, 540, DockEdge::Left, monitor, threshold));
    assert!(!is_cursor_at_edge(20, 540, DockEdge::Left, monitor, threshold));

    // Right edge: x near 1920
    assert!(is_cursor_at_edge(1920, 540, DockEdge::Right, monitor, threshold));
    assert!(is_cursor_at_edge(1917, 540, DockEdge::Right, monitor, threshold));
    assert!(!is_cursor_at_edge(1900, 540, DockEdge::Right, monitor, threshold));
}

#[test]
fn test_is_cursor_in_rect() {
    let rect = BarRect::new(0, 1032, 1920, 1080);
    assert!(is_cursor_in_rect(100, 1050, rect));
    assert!(is_cursor_in_rect(0, 1032, rect));
    assert!(is_cursor_in_rect(1920, 1080, rect));
    assert!(!is_cursor_in_rect(100, 1000, rect));
    assert!(!is_cursor_in_rect(2000, 1050, rect));
}

// ============================================================================
// 5. Auto-Hide State Transitions & Controller Lifecycle Tests
// ============================================================================

#[test]
fn test_autohide_state_variants_and_transitions() {
    let initial = AutoHideState::Hidden;
    assert!(initial.is_hidden());
    assert!(!initial.is_visible());

    // Hover triggers transition to Visible
    let revealed = initial.transition(true);
    assert_eq!(revealed, AutoHideState::Visible);
    assert!(revealed.is_visible());
    assert!(!revealed.is_hidden());

    // Inactivity triggers transition back to Hidden
    let concealed = revealed.transition(false);
    assert_eq!(concealed, AutoHideState::Hidden);
    assert!(concealed.is_hidden());

    // Animation sub-states
    let showing = AutoHideState::Showing;
    assert!(showing.is_visible());
    let hiding = AutoHideState::Hiding;
    assert!(hiding.is_hidden());
}

#[test]
fn test_autohide_controller_edge_hover_lifecycle() {
    let monitor = BarRect::new(0, 0, 1920, 1080);
    let mut controller = AutoHideController::new(
        DockEdge::Bottom,
        monitor,
        48,
        DEFAULT_AUTO_HIDE_PEEK,
        DEFAULT_EDGE_TRIGGER_THRESHOLD,
    );

    // Initial state: hidden off-screen
    assert_eq!(controller.state(), AutoHideState::Hidden);
    assert_eq!(controller.current_rect(), controller.hidden_rect());

    // Cursor at screen center (work area): remains hidden
    let state = controller.update_cursor(960, 540);
    assert_eq!(state, AutoHideState::Hidden);
    assert_eq!(controller.current_rect(), controller.hidden_rect());

    // Cursor moves to bottom screen edge (hover trigger zone): reveals bar
    let state = controller.update_cursor(960, 1079);
    assert_eq!(state, AutoHideState::Visible);
    assert_eq!(controller.current_rect(), controller.visible_rect());

    // Cursor now rests inside the taskbar body (y = 1050): remains visible
    let state = controller.update_cursor(960, 1050);
    assert_eq!(state, AutoHideState::Visible);
    assert_eq!(controller.current_rect(), controller.visible_rect());

    // Cursor moves back out into desktop (y = 800): conceals bar back off-screen
    let state = controller.update_cursor(960, 800);
    assert_eq!(state, AutoHideState::Hidden);
    assert_eq!(controller.current_rect(), controller.hidden_rect());
}

// ============================================================================
// 6. Native Tray Toggle & Flyout Discovery Tests
// ============================================================================

#[test]
fn test_find_native_tray_notify_window_safety() {
    // Verifies find_tray_notify_window() runs cleanly without panicking
    let _tray_notify = find_tray_notify_window();
    let _shell_tray = find_native_tray_window();
}

#[test]
fn test_toggle_native_tray_execution() {
    let has_tray = find_native_tray_window().is_some();
    if has_tray {
        let was_visible = is_native_tray_visible();
        let toggled = toggle_native_tray();
        assert_ne!(
            was_visible, toggled,
            "Toggling tray must invert current visibility"
        );
        let restored = toggle_native_tray();
        assert_eq!(
            was_visible, restored,
            "Toggling twice must restore original visibility"
        );
    } else {
        // Headless / test environment fallback
        assert!(!toggle_native_tray());
    }
}
