use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    SetWindowPos, HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
};

use crate::config::DockEdge;
use crate::geometry::BarRect;

/// Default pixel sliver kept visible on-screen during auto-hide to receive hover hit-tests.
pub const DEFAULT_AUTO_HIDE_PEEK: i32 = 2;

/// Default pixel boundary along the screen edge that triggers auto-hide taskbar reveal.
pub const DEFAULT_EDGE_TRIGGER_THRESHOLD: i32 = 4;

/// Represents the visibility and animation lifecycle state of an auto-hiding taskbar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AutoHideState {
    /// Taskbar is concealed off-screen.
    #[default]
    Hidden,
    /// Taskbar is animating into view.
    Showing,
    /// Taskbar is fully visible on-screen.
    Visible,
    /// Taskbar is animating out of view.
    Hiding,
}

impl AutoHideState {
    /// Returns true if the taskbar is fully visible or actively animating into view.
    pub fn is_visible(&self) -> bool {
        matches!(self, Self::Visible | Self::Showing)
    }

    /// Returns true if the taskbar is fully concealed or actively animating out of view.
    pub fn is_hidden(&self) -> bool {
        matches!(self, Self::Hidden | Self::Hiding)
    }

    /// Transitions the state directly based on hover status.
    ///
    /// When `is_hovered` is true, transitions to [`AutoHideState::Visible`].
    /// When `is_hovered` is false, transitions to [`AutoHideState::Hidden`].
    pub fn transition(&self, is_hovered: bool) -> Self {
        if is_hovered {
            Self::Visible
        } else {
            Self::Hidden
        }
    }

    /// Transitions into the [`AutoHideState::Showing`] animation sub-state.
    pub fn animate_in(&self) -> Self {
        Self::Showing
    }

    /// Transitions into the [`AutoHideState::Hiding`] animation sub-state.
    pub fn animate_out(&self) -> Self {
        Self::Hiding
    }

    /// Completes any in-flight animation transition to its resting state.
    pub fn complete_transition(&self) -> Self {
        match self {
            Self::Showing => Self::Visible,
            Self::Hiding => Self::Hidden,
            other => *other,
        }
    }
}

/// Computes the concealed off-screen rectangle for an AppBar docked along any of the 4 edges.
///
/// An optional `peek` value specifies how many pixels of the bar remain visible along the screen
/// boundary (for hit testing or visual sliver). Set `peek = 0` to conceal the bar completely off-screen.
pub fn calculate_hidden_rect(edge: DockEdge, visible_rect: BarRect, peek: i32) -> BarRect {
    let peek = peek.max(0);
    match edge {
        DockEdge::Bottom => {
            let height = visible_rect.height();
            let offset = (height - peek).max(0);
            BarRect {
                left: visible_rect.left,
                top: visible_rect.top + offset,
                right: visible_rect.right,
                bottom: visible_rect.bottom + offset,
            }
        }
        DockEdge::Top => {
            let height = visible_rect.height();
            let offset = (height - peek).max(0);
            BarRect {
                left: visible_rect.left,
                top: visible_rect.top - offset,
                right: visible_rect.right,
                bottom: visible_rect.bottom - offset,
            }
        }
        DockEdge::Left => {
            let width = visible_rect.width();
            let offset = (width - peek).max(0);
            BarRect {
                left: visible_rect.left - offset,
                top: visible_rect.top,
                right: visible_rect.right - offset,
                bottom: visible_rect.bottom,
            }
        }
        DockEdge::Right => {
            let width = visible_rect.width();
            let offset = (width - peek).max(0);
            BarRect {
                left: visible_rect.left + offset,
                top: visible_rect.top,
                right: visible_rect.right + offset,
                bottom: visible_rect.bottom,
            }
        }
    }
}

/// Determines whether mouse cursor coordinates fall within the activation trigger zone
/// along the specified screen docking edge.
pub fn is_cursor_at_edge(
    cursor_x: i32,
    cursor_y: i32,
    edge: DockEdge,
    monitor_rect: BarRect,
    threshold: i32,
) -> bool {
    let t = threshold.max(1);
    match edge {
        DockEdge::Bottom => {
            cursor_x >= monitor_rect.left
                && cursor_x <= monitor_rect.right
                && cursor_y >= monitor_rect.bottom - t
                && cursor_y <= monitor_rect.bottom + t
        }
        DockEdge::Top => {
            cursor_x >= monitor_rect.left
                && cursor_x <= monitor_rect.right
                && cursor_y >= monitor_rect.top - t
                && cursor_y <= monitor_rect.top + t
        }
        DockEdge::Left => {
            cursor_y >= monitor_rect.top
                && cursor_y <= monitor_rect.bottom
                && cursor_x >= monitor_rect.left - t
                && cursor_x <= monitor_rect.left + t
        }
        DockEdge::Right => {
            cursor_y >= monitor_rect.top
                && cursor_y <= monitor_rect.bottom
                && cursor_x >= monitor_rect.right - t
                && cursor_x <= monitor_rect.right + t
        }
    }
}

/// Checks whether cursor coordinates fall within the given rectangle bounds.
pub fn is_cursor_in_rect(cursor_x: i32, cursor_y: i32, rect: BarRect) -> bool {
    cursor_x >= rect.left
        && cursor_x <= rect.right
        && cursor_y >= rect.top
        && cursor_y <= rect.bottom
}

/// Evaluates whether a given window rectangle fully covers or exceeds monitor dimensions,
/// correctly classifying exclusive fullscreen and borderless fullscreen windows.
pub fn is_rect_fullscreen(window_rect: BarRect, monitor_rect: BarRect) -> bool {
    if window_rect.width() <= 0 || window_rect.height() <= 0 {
        return false;
    }
    if monitor_rect.width() <= 0 || monitor_rect.height() <= 0 {
        return false;
    }
    window_rect.left <= monitor_rect.left
        && window_rect.top <= monitor_rect.top
        && window_rect.right >= monitor_rect.right
        && window_rect.bottom >= monitor_rect.bottom
}

/// Sets a window's z-order to either topmost ([`HWND_TOPMOST`]) or normal non-topmost ([`HWND_NOTOPMOST`]).
///
/// Used by Fullscreen Yield to lower the taskbar beneath fullscreen games and media players,
/// and re-engage topmost docking when fullscreen applications lose focus or exit.
pub fn set_window_topmost(hwnd: isize, topmost: bool) -> bool {
    if hwnd == 0 {
        return false;
    }
    unsafe {
        let h = HWND(hwnd as *mut _);
        let insert_after = if topmost { HWND_TOPMOST } else { HWND_NOTOPMOST };
        SetWindowPos(
            h,
            insert_after,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        )
        .is_ok()
    }
}

/// Sets the taskbar window's topmost z-order status. Alias for [`set_window_topmost`].
pub fn set_taskbar_topmost(hwnd: isize, topmost: bool) -> bool {
    set_window_topmost(hwnd, topmost)
}

/// Coordinates auto-hide state transitions, edge hover trigger detection, and bounding box geometry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutoHideController {
    state: AutoHideState,
    edge: DockEdge,
    visible_rect: BarRect,
    hidden_rect: BarRect,
    monitor_rect: BarRect,
    threshold: i32,
}

impl AutoHideController {
    /// Constructs a new `AutoHideController` for the specified dock edge and monitor bounds.
    pub fn new(
        edge: DockEdge,
        monitor_rect: BarRect,
        size: u32,
        peek: i32,
        threshold: i32,
    ) -> Self {
        let visible_rect = crate::geometry::calculate_appbar_rect(edge, size, monitor_rect);
        let hidden_rect = calculate_hidden_rect(edge, visible_rect, peek);
        Self {
            state: AutoHideState::Hidden,
            edge,
            visible_rect,
            hidden_rect,
            monitor_rect,
            threshold,
        }
    }

    /// Returns the current auto-hide visibility state.
    pub fn state(&self) -> AutoHideState {
        self.state
    }

    /// Returns true if the taskbar is currently revealed or animating into view.
    pub fn is_visible(&self) -> bool {
        self.state.is_visible()
    }

    /// Returns true if the taskbar is currently concealed or animating out of view.
    pub fn is_hidden(&self) -> bool {
        self.state.is_hidden()
    }

    /// Returns the normal, fully visible AppBar bounding rectangle.
    pub fn visible_rect(&self) -> BarRect {
        self.visible_rect
    }

    /// Returns the concealed off-screen AppBar bounding rectangle.
    pub fn hidden_rect(&self) -> BarRect {
        self.hidden_rect
    }

    /// Returns the active bounding rectangle corresponding to the current auto-hide state.
    pub fn current_rect(&self) -> BarRect {
        match self.state {
            AutoHideState::Visible | AutoHideState::Showing => self.visible_rect,
            AutoHideState::Hidden | AutoHideState::Hiding => self.hidden_rect,
        }
    }

    /// Updates cursor position and evaluates state transitions.
    ///
    /// If the cursor hovers within the edge trigger zone or over the revealed bar,
    /// the state becomes [`AutoHideState::Visible`]. When the cursor moves away,
    /// it transitions to [`AutoHideState::Hidden`].
    pub fn update_cursor(&mut self, cursor_x: i32, cursor_y: i32) -> AutoHideState {
        let at_edge = is_cursor_at_edge(
            cursor_x,
            cursor_y,
            self.edge,
            self.monitor_rect,
            self.threshold,
        );
        let in_bar = is_cursor_in_rect(cursor_x, cursor_y, self.visible_rect);
        let hover = at_edge || (self.state.is_visible() && in_bar);

        self.state = if hover {
            AutoHideState::Visible
        } else {
            AutoHideState::Hidden
        };
        self.state
    }

    /// Manually overrides the current auto-hide state.
    pub fn force_state(&mut self, state: AutoHideState) {
        self.state = state;
    }
}
