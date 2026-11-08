use taskbar_clone::config::DockEdge;
use taskbar_clone::geometry::{calculate_appbar_rect, BarRect};

#[test]
fn test_appbar_rect_calculations_primary_monitor() {
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
    assert_eq!(bottom.height(), 48);
    assert_eq!(bottom.width(), 1920);

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
    assert_eq!(top.height(), 48);
    assert_eq!(top.width(), 1920);

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
    assert_eq!(left.width(), 48);
    assert_eq!(left.height(), 1080);

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
    assert_eq!(right.width(), 48);
    assert_eq!(right.height(), 1080);
}

#[test]
fn test_appbar_rect_calculations_offset_monitor() {
    // Secondary monitor positioned to the left and slightly offset vertically
    let mon = BarRect {
        left: -1920,
        top: -120,
        right: 0,
        bottom: 960,
    };
    let size = 56;

    let bottom = calculate_appbar_rect(DockEdge::Bottom, size, mon);
    assert_eq!(
        bottom,
        BarRect {
            left: -1920,
            top: 904,
            right: 0,
            bottom: 960
        }
    );

    let top = calculate_appbar_rect(DockEdge::Top, size, mon);
    assert_eq!(
        top,
        BarRect {
            left: -1920,
            top: -120,
            right: 0,
            bottom: -64
        }
    );

    let left = calculate_appbar_rect(DockEdge::Left, size, mon);
    assert_eq!(
        left,
        BarRect {
            left: -1920,
            top: -120,
            right: -1864,
            bottom: 960
        }
    );

    let right = calculate_appbar_rect(DockEdge::Right, size, mon);
    assert_eq!(
        right,
        BarRect {
            left: -56,
            top: -120,
            right: 0,
            bottom: 960
        }
    );
}

#[test]
fn test_edge_orientation() {
    assert!(!DockEdge::Bottom.is_vertical());
    assert!(DockEdge::Bottom.is_horizontal());

    assert!(!DockEdge::Top.is_vertical());
    assert!(DockEdge::Top.is_horizontal());

    assert!(DockEdge::Left.is_vertical());
    assert!(!DockEdge::Left.is_horizontal());

    assert!(DockEdge::Right.is_vertical());
    assert!(!DockEdge::Right.is_horizontal());
}

#[test]
fn test_bar_rect_helpers_and_win32_conversion() {
    let rect = BarRect::new(10, 20, 110, 220);
    assert_eq!(rect.width(), 100);
    assert_eq!(rect.height(), 200);

    let win_rect: windows::Win32::Foundation::RECT = rect.into();
    assert_eq!(win_rect.left, 10);
    assert_eq!(win_rect.top, 20);
    assert_eq!(win_rect.right, 110);
    assert_eq!(win_rect.bottom, 220);

    let roundtrip: BarRect = win_rect.into();
    assert_eq!(roundtrip, rect);
}
