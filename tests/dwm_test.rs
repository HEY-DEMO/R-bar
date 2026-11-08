use taskbar_clone::config::DockEdge;
use taskbar_clone::dwm::{
    calculate_thumbnail_aspect_fit, create_dwm_thumbnail, unregister_dwm_thumbnail,
    update_dwm_thumbnail, DwmThumbnailConfig, DwmThumbnailSession, PreviewClickAction,
    PreviewPopup, PreviewPopupController, PreviewTile,
};
use taskbar_clone::geometry::BarRect;
use taskbar_clone::tasks::{GroupedTask, TaskItem};

#[test]
fn test_thumbnail_session_zero_handle() {
    let session = DwmThumbnailSession::new(0, 0, 0, BarRect::default());
    assert_eq!(session.handle, 0);
    assert!(!session.is_valid());
    // Dropping zero-handle session must not crash or invoke OS unregister
    drop(session);

    // Direct struct initialization with zero handle as specified in project plan
    let session_struct = DwmThumbnailSession {
        handle: 0,
        ..Default::default()
    };
    assert_eq!(session_struct.handle, 0);
    assert!(!session_struct.is_valid());
    drop(session_struct);
}

#[test]
fn test_thumbnail_session_lifecycle_and_drop() {
    // Unregistering 0 returns false safely
    assert!(!unregister_dwm_thumbnail(0));

    let mut session = DwmThumbnailSession::new(12345, 100, 200, BarRect::new(10, 10, 190, 130));
    assert_eq!(session.handle, 12345);
    assert!(session.is_valid());
    assert_eq!(session.dest_hwnd, 100);
    assert_eq!(session.src_hwnd, 200);
    assert_eq!(session.dest_rect, BarRect::new(10, 10, 190, 130));

    // Manual unregister clears the handle
    session.unregister();
    assert_eq!(session.handle, 0);
    assert!(!session.is_valid());

    // Subsequent drop is safe (handle already 0)
    drop(session);
}

#[test]
fn test_thumbnail_properties_config() {
    let rect = BarRect::new(20, 20, 220, 160);
    let config = DwmThumbnailConfig::new(rect)
        .with_opacity(200)
        .with_visible(true)
        .with_client_area_only(true);

    assert_eq!(config.dest_rect, rect);
    assert_eq!(config.opacity, 200);
    assert!(config.visible);
    assert!(config.client_area_only);
    assert!(config.source_rect.is_none());

    let src_rect = BarRect::new(0, 0, 800, 600);
    let config_with_src = config.with_source_rect(src_rect);
    assert_eq!(config_with_src.source_rect, Some(src_rect));
}

#[test]
fn test_thumbnail_aspect_ratio_fitting() {
    let target_box = BarRect::new(10, 10, 210, 130); // 200w x 120h
    assert_eq!(target_box.width(), 200);
    assert_eq!(target_box.height(), 120);

    // 1. 16:9 Landscape (1920x1080): aspect ratio ~ 1.778
    // Target aspect ratio: 200/120 ~ 1.667. Source is wider than target.
    // Fit should constrain by width (200), resulting height = 200 * 1080 / 1920 = 112
    // Top offset = 10 + (120 - 112) / 2 = 14
    let fitted_16_9 = calculate_thumbnail_aspect_fit((1920, 1080), target_box);
    assert_eq!(fitted_16_9.width(), 200);
    assert_eq!(fitted_16_9.height(), 112);
    assert_eq!(fitted_16_9.left, 10);
    assert_eq!(fitted_16_9.top, 14);

    // 2. Portrait (1080x1920): source is taller than target
    // Constrained by height (120), resulting width = 120 * 1080 / 1920 = 67
    // Left offset = 10 + (200 - 67) / 2 = 76
    let fitted_portrait = calculate_thumbnail_aspect_fit((1080, 1920), target_box);
    assert_eq!(fitted_portrait.height(), 120);
    assert_eq!(fitted_portrait.width(), 67);
    assert_eq!(fitted_portrait.top, 10);
    assert_eq!(fitted_portrait.left, 76);

    // 3. Fallback for invalid / zero source dimensions: should fill target box
    let fitted_zero = calculate_thumbnail_aspect_fit((0, 0), target_box);
    assert_eq!(fitted_zero, target_box);
}

#[test]
fn test_preview_tile_layout_single_window() {
    let task = GroupedTask {
        app_id: "notepad.exe".into(),
        title: "Untitled - Notepad".into(),
        windows: vec![TaskItem {
            hwnd: 1001,
            pid: 501,
            title: "Untitled - Notepad".into(),
            app_id: "notepad.exe".into(),
            is_active: true,
            is_minimized: false,
        }],
    };

    let anchor = BarRect::new(100, 1032, 148, 1080);
    let mon = BarRect::new(0, 0, 1920, 1080);

    let popup = PreviewPopup::calculate_layout(&task, anchor, DockEdge::Bottom, mon);
    assert_eq!(popup.tiles.len(), 1);
    assert_eq!(popup.hovered_app_id, "notepad.exe");

    let tile: &PreviewTile = &popup.tiles[0];
    assert_eq!(tile.hwnd, 1001);
    assert_eq!(tile.title, "Untitled - Notepad");
    assert!(tile.bounds.width() >= 160);
    assert!(tile.bounds.height() >= 100);
    assert!(tile.thumbnail_rect.width() > 0);
    assert!(tile.thumbnail_rect.height() > 0);
    assert!(tile.close_button_rect.width() > 0);
    assert!(tile.close_button_rect.height() > 0);

    // Client coordinates relative to popup window
    let client_bounds = tile.client_bounds(&popup.bounds);
    assert_eq!(client_bounds.left, tile.bounds.left - popup.bounds.left);
    assert_eq!(client_bounds.top, tile.bounds.top - popup.bounds.top);
    assert_eq!(client_bounds.width(), tile.bounds.width());
    assert_eq!(client_bounds.height(), tile.bounds.height());

    let client_thumb = tile.client_thumbnail_rect(&popup.bounds);
    assert!(client_thumb.left >= client_bounds.left);
    assert!(client_thumb.right <= client_bounds.right);

    let client_close = tile.client_close_button_rect(&popup.bounds);
    assert!(client_close.left >= client_bounds.left);
    assert!(client_close.right <= client_bounds.right);

    // Popup bounds should be above taskbar (Bottom edge: popup bottom <= anchor top)
    assert!(popup.bounds.bottom <= anchor.top);
    assert!(popup.bounds.left >= mon.left);
    assert!(popup.bounds.right <= mon.right);
}

#[test]
fn test_preview_tile_layout_multi_window() {
    let task = GroupedTask {
        app_id: "msedge.exe".into(),
        title: "Edge".into(),
        windows: vec![
            TaskItem {
                hwnd: 2001,
                pid: 601,
                title: "GitHub - Pull Request".into(),
                app_id: "msedge.exe".into(),
                is_active: false,
                is_minimized: false,
            },
            TaskItem {
                hwnd: 2002,
                pid: 601,
                title: "Rust Documentation".into(),
                app_id: "msedge.exe".into(),
                is_active: true,
                is_minimized: false,
            },
            TaskItem {
                hwnd: 2003,
                pid: 601,
                title: "Stack Overflow".into(),
                app_id: "msedge.exe".into(),
                is_active: false,
                is_minimized: true,
            },
        ],
    };

    let anchor = BarRect::new(400, 1032, 448, 1080);
    let mon = BarRect::new(0, 0, 1920, 1080);

    let popup = PreviewPopup::calculate_layout(&task, anchor, DockEdge::Bottom, mon);
    assert_eq!(popup.tiles.len(), 3);

    // Tiles should be laid out horizontally in order with non-overlapping bounds
    for i in 0..popup.tiles.len() {
        assert_eq!(popup.tiles[i].hwnd, task.windows[i].hwnd);
        assert_eq!(popup.tiles[i].title, task.windows[i].title);
        if i > 0 {
            assert!(
                popup.tiles[i].bounds.left >= popup.tiles[i - 1].bounds.right,
                "Tile {} should be placed to the right of tile {}",
                i,
                i - 1
            );
        }
    }
}

#[test]
fn test_preview_popup_docking_all_edges() {
    let task = GroupedTask {
        app_id: "test.exe".into(),
        title: "Test".into(),
        windows: vec![TaskItem {
            hwnd: 3001,
            pid: 701,
            title: "Test Window".into(),
            app_id: "test.exe".into(),
            is_active: false,
            is_minimized: false,
        }],
    };
    let mon = BarRect::new(0, 0, 1920, 1080);

    // 1. Bottom edge: popup placed above bar
    let anchor_bottom = BarRect::new(500, 1032, 548, 1080);
    let popup_bottom = PreviewPopup::calculate_layout(&task, anchor_bottom, DockEdge::Bottom, mon);
    assert!(popup_bottom.bounds.bottom <= anchor_bottom.top);

    // 2. Top edge: popup placed below bar
    let anchor_top = BarRect::new(500, 0, 548, 48);
    let popup_top = PreviewPopup::calculate_layout(&task, anchor_top, DockEdge::Top, mon);
    assert!(popup_top.bounds.top >= anchor_top.bottom);

    // 3. Left edge: popup placed to the right of bar
    let anchor_left = BarRect::new(0, 500, 48, 548);
    let popup_left = PreviewPopup::calculate_layout(&task, anchor_left, DockEdge::Left, mon);
    assert!(popup_left.bounds.left >= anchor_left.right);

    // 4. Right edge: popup placed to the left of bar
    let anchor_right = BarRect::new(1872, 500, 1920, 548);
    let popup_right = PreviewPopup::calculate_layout(&task, anchor_right, DockEdge::Right, mon);
    assert!(popup_right.bounds.right <= anchor_right.left);
}

#[test]
fn test_preview_popup_click_actions() {
    let task = GroupedTask {
        app_id: "code.exe".into(),
        title: "VS Code".into(),
        windows: vec![TaskItem {
            hwnd: 4001,
            pid: 801,
            title: "main.rs - Project".into(),
            app_id: "code.exe".into(),
            is_active: false,
            is_minimized: false,
        }],
    };
    let anchor = BarRect::new(200, 1032, 248, 1080);
    let mon = BarRect::new(0, 0, 1920, 1080);
    let popup = PreviewPopup::calculate_layout(&task, anchor, DockEdge::Bottom, mon);

    let tile = &popup.tiles[0];

    // Click inside close button -> CloseWindow(hwnd)
    let close_click_x = tile.close_button_rect.left + tile.close_button_rect.width() / 2;
    let close_click_y = tile.close_button_rect.top + tile.close_button_rect.height() / 2;
    let action_close = popup.handle_click(close_click_x, close_click_y);
    assert_eq!(action_close, Some(PreviewClickAction::CloseWindow(4001)));

    // Click inside thumbnail preview -> ActivateWindow(hwnd)
    let thumb_click_x = tile.thumbnail_rect.left + tile.thumbnail_rect.width() / 2;
    let thumb_click_y = tile.thumbnail_rect.top + tile.thumbnail_rect.height() / 2;
    let action_thumb = popup.handle_click(thumb_click_x, thumb_click_y);
    assert_eq!(action_thumb, Some(PreviewClickAction::ActivateWindow(4001)));

    // Click outside any tile -> None
    let action_outside = popup.handle_click(popup.bounds.right + 50, popup.bounds.bottom + 50);
    assert_eq!(action_outside, None);
}

#[test]
fn test_preview_popup_controller_lifecycle() {
    let mut controller = PreviewPopupController::new();
    assert!(!controller.is_active());
    assert!(controller.current_popup().is_none());

    let task = GroupedTask {
        app_id: "terminal.exe".into(),
        title: "Windows Terminal".into(),
        windows: vec![
            TaskItem {
                hwnd: 5001,
                pid: 901,
                title: "PowerShell".into(),
                app_id: "terminal.exe".into(),
                is_active: true,
                is_minimized: false,
            },
            TaskItem {
                hwnd: 5002,
                pid: 901,
                title: "Command Prompt".into(),
                app_id: "terminal.exe".into(),
                is_active: false,
                is_minimized: false,
            },
        ],
    };
    let anchor = BarRect::new(300, 1032, 348, 1080);
    let mon = BarRect::new(0, 0, 1920, 1080);

    // Hover starts preview
    controller.show_for_task(&task, anchor, DockEdge::Bottom, mon);
    assert!(controller.is_active());
    let current = controller.current_popup().expect("popup should be present");
    assert_eq!(current.tiles.len(), 2);
    assert_eq!(current.hovered_app_id, "terminal.exe");

    // Hover detection
    let center_x = current.bounds.left + current.bounds.width() / 2;
    let center_y = current.bounds.top + current.bounds.height() / 2;
    assert!(controller.is_hovered(center_x, center_y, anchor));
    assert!(controller.is_hovered(anchor.left + 5, anchor.top + 5, anchor));
    assert!(!controller.is_hovered(0, 0, anchor));

    // Close preview
    controller.hide();
    assert!(!controller.is_active());
    assert!(controller.current_popup().is_none());
}

#[test]
fn test_real_dwm_thumbnail_registration_with_mock_or_test_windows() {
    #[cfg(windows)]
    {
        use windows::core::{w, PCWSTR};
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{
            CreateWindowExW, DestroyWindow, WINDOW_EX_STYLE, WS_POPUP,
        };

        // Create two hidden windows: one destination (popup) and one source (target window)
        let dest_hwnd = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("STATIC"),
                PCWSTR(std::ptr::null()),
                WS_POPUP,
                0,
                0,
                300,
                200,
                HWND(std::ptr::null_mut()),
                None,
                None,
                None,
            )
        };

        let src_hwnd = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("STATIC"),
                PCWSTR(std::ptr::null()),
                WS_POPUP,
                0,
                0,
                400,
                300,
                HWND(std::ptr::null_mut()),
                None,
                None,
                None,
            )
        };

        if let (Ok(dest), Ok(src)) = (dest_hwnd, src_hwnd) {
            let dest_val = dest.0 as isize;
            let src_val = src.0 as isize;

            // Attempt DwmRegisterThumbnail
            let dest_rect = BarRect::new(10, 10, 200, 150);
            let session = create_dwm_thumbnail(dest_val, src_val, dest_rect);

            if let Some(mut session) = session {
                assert!(session.is_valid());
                assert_ne!(session.handle, 0);

                // Update properties
                let ok = update_dwm_thumbnail(session.handle, BarRect::new(20, 20, 220, 170), 220, true);
                assert!(ok);

                // Explicit unregister
                session.unregister();
                assert_eq!(session.handle, 0);
            }

            unsafe {
                let _ = DestroyWindow(dest);
                let _ = DestroyWindow(src);
            }
        }
    }
}

#[test]
fn test_real_preview_popup_native_window_lifecycle() {
    let task = GroupedTask {
        app_id: "testapp.exe".into(),
        title: "Test App".into(),
        windows: vec![TaskItem {
            hwnd: 6001,
            pid: 1001,
            title: "Test App Window".into(),
            app_id: "testapp.exe".into(),
            is_active: false,
            is_minimized: false,
        }],
    };
    let anchor = BarRect::new(200, 1032, 248, 1080);
    let mon = BarRect::new(0, 0, 1920, 1080);
    let mut popup = PreviewPopup::calculate_layout(&task, anchor, DockEdge::Bottom, mon);

    assert_eq!(popup.hwnd, 0);
    let shown = popup.show_native_window();
    if shown {
        assert_ne!(popup.hwnd, 0);
        let hidden = popup.hide_native_window();
        assert!(hidden);
        assert_eq!(popup.hwnd, 0);
    }
}
