use crate::config::DockEdge;

/// Represents a rectangle in screen coordinates (pixels).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BarRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl BarRect {
    pub const fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    pub fn width(&self) -> i32 {
        self.right - self.left
    }

    pub fn height(&self) -> i32 {
        self.bottom - self.top
    }
}

#[cfg(windows)]
impl From<windows::Win32::Foundation::RECT> for BarRect {
    fn from(r: windows::Win32::Foundation::RECT) -> Self {
        Self {
            left: r.left,
            top: r.top,
            right: r.right,
            bottom: r.bottom,
        }
    }
}

#[cfg(windows)]
impl From<BarRect> for windows::Win32::Foundation::RECT {
    fn from(r: BarRect) -> Self {
        Self {
            left: r.left,
            top: r.top,
            right: r.right,
            bottom: r.bottom,
        }
    }
}

/// Computes the exact window coordinate rectangle for an AppBar docked to any of the 4 screen edges,
/// based on the monitor bounds and the configured bar thickness/size.
pub fn calculate_appbar_rect(edge: DockEdge, size: u32, monitor_rect: BarRect) -> BarRect {
    let size = size as i32;
    match edge {
        DockEdge::Bottom => BarRect {
            left: monitor_rect.left,
            top: monitor_rect.bottom - size,
            right: monitor_rect.right,
            bottom: monitor_rect.bottom,
        },
        DockEdge::Top => BarRect {
            left: monitor_rect.left,
            top: monitor_rect.top,
            right: monitor_rect.right,
            bottom: monitor_rect.top + size,
        },
        DockEdge::Left => BarRect {
            left: monitor_rect.left,
            top: monitor_rect.top,
            right: monitor_rect.left + size,
            bottom: monitor_rect.bottom,
        },
        DockEdge::Right => BarRect {
            left: monitor_rect.right - size,
            top: monitor_rect.top,
            right: monitor_rect.right,
            bottom: monitor_rect.bottom,
        },
    }
}
