use std::sync::Once;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{BOOL, HINSTANCE, HWND, LPARAM, LRESULT, RECT, SIZE, WPARAM};
use windows::Win32::Graphics::Dwm::{
    DwmQueryThumbnailSourceSize, DwmRegisterThumbnail, DwmUnregisterThumbnail,
    DwmUpdateThumbnailProperties, DWM_THUMBNAIL_PROPERTIES, DWM_TNP_OPACITY,
    DWM_TNP_RECTDESTINATION, DWM_TNP_RECTSOURCE, DWM_TNP_SOURCECLIENTAREAONLY, DWM_TNP_VISIBLE,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, IsWindow, RegisterClassExW,
    WINDOW_EX_STYLE, WNDCLASSEXW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    WS_POPUP, WS_VISIBLE,
};

use crate::config::DockEdge;
use crate::geometry::BarRect;
use crate::tasks::GroupedTask;

pub const TILE_WIDTH: i32 = 200;
pub const TILE_HEIGHT: i32 = 130;
pub const TILE_PADDING: i32 = 8;
pub const POPUP_PADDING: i32 = 8;
pub const HEADER_HEIGHT: i32 = 26;
pub const CLOSE_BTN_SIZE: i32 = 20;
pub const POPUP_GAP: i32 = 4;

/// Configuration for updating DWM thumbnail composition properties.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DwmThumbnailConfig {
    pub dest_rect: BarRect,
    pub source_rect: Option<BarRect>,
    pub opacity: u8,
    pub visible: bool,
    pub client_area_only: bool,
}

impl DwmThumbnailConfig {
    pub fn new(dest_rect: BarRect) -> Self {
        Self {
            dest_rect,
            source_rect: None,
            opacity: 255,
            visible: true,
            client_area_only: false,
        }
    }

    pub fn with_source_rect(mut self, rect: BarRect) -> Self {
        self.source_rect = Some(rect);
        self
    }

    pub fn with_opacity(mut self, opacity: u8) -> Self {
        self.opacity = opacity;
        self
    }

    pub fn with_visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    pub fn with_client_area_only(mut self, client_only: bool) -> Self {
        self.client_area_only = client_only;
        self
    }
}

/// A RAII-managed session for a live Desktop Window Manager (DWM) hardware thumbnail composition.
///
/// When dropped, it automatically unregisters the DWM thumbnail composition to prevent resource leaks.
#[derive(Debug, Default)]
pub struct DwmThumbnailSession {
    pub handle: isize,
    pub dest_hwnd: isize,
    pub src_hwnd: isize,
    pub dest_rect: BarRect,
}

impl DwmThumbnailSession {
    pub fn new(handle: isize, dest_hwnd: isize, src_hwnd: isize, dest_rect: BarRect) -> Self {
        Self {
            handle,
            dest_hwnd,
            src_hwnd,
            dest_rect,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.handle != 0
    }

    pub fn unregister(&mut self) {
        if self.handle != 0 {
            unregister_dwm_thumbnail(self.handle);
            self.handle = 0;
        }
    }

    pub fn update_rect(&mut self, new_rect: BarRect) -> bool {
        if self.handle == 0 {
            return false;
        }
        let config = DwmThumbnailConfig::new(new_rect);
        if apply_thumbnail_properties(self.handle, &config) {
            self.dest_rect = new_rect;
            true
        } else {
            false
        }
    }
}

impl Drop for DwmThumbnailSession {
    fn drop(&mut self) {
        self.unregister();
    }
}

/// Registers a live DWM thumbnail compositing relationship between a destination window and a source window.
pub fn create_dwm_thumbnail(
    dest_hwnd: isize,
    src_hwnd: isize,
    dest_rect: BarRect,
) -> Option<DwmThumbnailSession> {
    if dest_hwnd == 0 || src_hwnd == 0 {
        return None;
    }

    unsafe {
        let dest = HWND(dest_hwnd as *mut _);
        let src = HWND(src_hwnd as *mut _);

        if !IsWindow(dest).as_bool() || !IsWindow(src).as_bool() {
            return None;
        }

        match DwmRegisterThumbnail(dest, src) {
            Ok(handle) => {
                let config = DwmThumbnailConfig::new(dest_rect);
                let _ = apply_thumbnail_properties(handle, &config);
                Some(DwmThumbnailSession::new(
                    handle, dest_hwnd, src_hwnd, dest_rect,
                ))
            }
            Err(e) => {
                log::warn!("DwmRegisterThumbnail failed for dest {dest_hwnd}, src {src_hwnd}: {e}");
                None
            }
        }
    }
}

/// Unregisters an active DWM thumbnail session.
pub fn unregister_dwm_thumbnail(handle: isize) -> bool {
    if handle == 0 {
        return false;
    }
    unsafe {
        match DwmUnregisterThumbnail(handle) {
            Ok(()) => true,
            Err(e) => {
                log::debug!("DwmUnregisterThumbnail error for {handle}: {e}");
                false
            }
        }
    }
}

/// Updates the composition properties of an existing DWM thumbnail.
pub fn update_dwm_thumbnail(
    handle: isize,
    dest_rect: BarRect,
    opacity: u8,
    visible: bool,
) -> bool {
    let config = DwmThumbnailConfig::new(dest_rect)
        .with_opacity(opacity)
        .with_visible(visible);
    apply_thumbnail_properties(handle, &config)
}

/// Applies a full `DwmThumbnailConfig` to a registered DWM thumbnail handle.
pub fn apply_thumbnail_properties(handle: isize, config: &DwmThumbnailConfig) -> bool {
    if handle == 0 {
        return false;
    }

    let mut flags = DWM_TNP_RECTDESTINATION | DWM_TNP_OPACITY | DWM_TNP_VISIBLE;
    let mut props = DWM_THUMBNAIL_PROPERTIES {
        rcDestination: RECT::from(config.dest_rect),
        opacity: config.opacity,
        fVisible: BOOL(if config.visible { 1 } else { 0 }),
        ..Default::default()
    };

    if let Some(src) = config.source_rect {
        flags |= DWM_TNP_RECTSOURCE;
        props.rcSource = RECT::from(src);
    }
    if config.client_area_only {
        flags |= DWM_TNP_SOURCECLIENTAREAONLY;
        props.fSourceClientAreaOnly = BOOL(1);
    }
    props.dwFlags = flags;

    unsafe { DwmUpdateThumbnailProperties(handle, &props).is_ok() }
}

/// Queries the original source window dimensions from the DWM thumbnail composition API.
pub fn query_dwm_thumbnail_source_size(handle: isize) -> Option<(i32, i32)> {
    if handle == 0 {
        return None;
    }
    unsafe {
        match DwmQueryThumbnailSourceSize(handle) {
            Ok(SIZE { cx, cy }) => Some((cx, cy)),
            Err(_) => None,
        }
    }
}

/// Calculates an aspect-ratio-preserving destination rectangle centered within `target_box`.
pub fn calculate_thumbnail_aspect_fit(source_size: (i32, i32), target_box: BarRect) -> BarRect {
    let (src_w, src_h) = source_size;
    let target_w = target_box.width();
    let target_h = target_box.height();

    if src_w <= 0 || src_h <= 0 || target_w <= 0 || target_h <= 0 {
        return target_box;
    }

    let (fit_w, fit_h) = if (src_w as i64 * target_h as i64) > (target_w as i64 * src_h as i64) {
        // Source is wider than target: fit to target width
        let w = target_w;
        let h = ((target_w as i64 * src_h as i64) / src_w as i64) as i32;
        (w, h.min(target_h))
    } else {
        // Source is taller than or equal to target: fit to target height
        let h = target_h;
        let w = ((target_h as i64 * src_w as i64) / src_h as i64) as i32;
        (w.min(target_w), h)
    };

    let offset_x = (target_w - fit_w) / 2;
    let offset_y = (target_h - fit_h) / 2;

    BarRect::new(
        target_box.left + offset_x,
        target_box.top + offset_y,
        target_box.left + offset_x + fit_w,
        target_box.top + offset_y + fit_h,
    )
}

/// Action triggered by interacting with a preview tile inside the hover popup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewClickAction {
    ActivateWindow(isize),
    CloseWindow(isize),
}

impl PreviewClickAction {
    /// Executes the click action against the operating system window.
    pub fn execute(&self) -> bool {
        match self {
            Self::ActivateWindow(hwnd) => {
                crate::tasks::activate_window(*hwnd);
                true
            }
            Self::CloseWindow(hwnd) => {
                crate::controls::execute_window_command(*hwnd, crate::controls::WindowCommand::Close)
            }
        }
    }
}

/// Represents a single window preview tile displayed within the hover popup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewTile {
    pub hwnd: isize,
    pub title: String,
    pub bounds: BarRect,
    pub thumbnail_rect: BarRect,
    pub close_button_rect: BarRect,
    pub is_active: bool,
    pub is_minimized: bool,
}

impl PreviewTile {
    pub fn client_bounds(&self, popup_bounds: &BarRect) -> BarRect {
        BarRect::new(
            self.bounds.left - popup_bounds.left,
            self.bounds.top - popup_bounds.top,
            self.bounds.right - popup_bounds.left,
            self.bounds.bottom - popup_bounds.top,
        )
    }

    pub fn client_thumbnail_rect(&self, popup_bounds: &BarRect) -> BarRect {
        BarRect::new(
            self.thumbnail_rect.left - popup_bounds.left,
            self.thumbnail_rect.top - popup_bounds.top,
            self.thumbnail_rect.right - popup_bounds.left,
            self.thumbnail_rect.bottom - popup_bounds.top,
        )
    }

    pub fn client_close_button_rect(&self, popup_bounds: &BarRect) -> BarRect {
        BarRect::new(
            self.close_button_rect.left - popup_bounds.left,
            self.close_button_rect.top - popup_bounds.top,
            self.close_button_rect.right - popup_bounds.left,
            self.close_button_rect.bottom - popup_bounds.top,
        )
    }
}

static REGISTER_PREVIEW_CLASS_ONCE: Once = Once::new();

unsafe extern "system" fn preview_window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

fn ensure_preview_window_class_registered() {
    REGISTER_PREVIEW_CLASS_ONCE.call_once(|| {
        let class_name = w!("TaskbarClone_PreviewWindowClass");
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(preview_window_proc),
            hInstance: HINSTANCE(std::ptr::null_mut()),
            lpszClassName: class_name,
            ..Default::default()
        };
        unsafe { RegisterClassExW(&wc) };
    });
}

/// Creates a top-level tool window popup for displaying live DWM thumbnails.
pub fn create_preview_window(bounds: BarRect) -> Option<isize> {
    ensure_preview_window_class_registered();
    let class_name = w!("TaskbarClone_PreviewWindowClass");
    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(WS_EX_TOPMOST.0 | WS_EX_TOOLWINDOW.0 | WS_EX_NOACTIVATE.0),
            class_name,
            PCWSTR(std::ptr::null()),
            WS_POPUP | WS_VISIBLE,
            bounds.left,
            bounds.top,
            bounds.width(),
            bounds.height(),
            HWND(std::ptr::null_mut()),
            None,
            HINSTANCE(std::ptr::null_mut()),
            None,
        )
    };
    hwnd.ok().map(|h| h.0 as isize)
}

/// Destroys a preview window created with `create_preview_window`.
pub fn destroy_preview_window(hwnd: isize) -> bool {
    if hwnd == 0 {
        return false;
    }
    unsafe { DestroyWindow(HWND(hwnd as *mut _)).is_ok() }
}

/// Represents the hover preview popup containing one or more preview tiles.
#[derive(Debug, Default)]
pub struct PreviewPopup {
    pub hwnd: isize,
    pub bounds: BarRect,
    pub tiles: Vec<PreviewTile>,
    pub hovered_app_id: String,
    pub hovered_tile_index: Option<usize>,
    pub sessions: Vec<DwmThumbnailSession>,
}

impl PreviewPopup {
    /// Calculates the layout for the preview popup and each window tile based on the task,
    /// anchor rectangle on the taskbar, dock edge, and monitor work area.
    pub fn calculate_layout(
        task: &GroupedTask,
        anchor_rect: BarRect,
        edge: DockEdge,
        mon_rect: BarRect,
    ) -> Self {
        let n = task.windows.len().max(1) as i32;
        let tiles_width = n * TILE_WIDTH + (n - 1) * TILE_PADDING;
        let total_width = tiles_width + 2 * POPUP_PADDING;
        let total_height = TILE_HEIGHT + 2 * POPUP_PADDING;

        let (left, top, right, bottom) = match edge {
            DockEdge::Bottom => {
                let top = anchor_rect.top - POPUP_GAP - total_height;
                let bottom = top + total_height;
                let mut left = anchor_rect.left + (anchor_rect.width() - total_width) / 2;
                let min_left = mon_rect.left + 4;
                let max_left = (mon_rect.right - total_width - 4).max(min_left);
                left = left.clamp(min_left, max_left);
                (left, top, left + total_width, bottom)
            }
            DockEdge::Top => {
                let top = anchor_rect.bottom + POPUP_GAP;
                let bottom = top + total_height;
                let mut left = anchor_rect.left + (anchor_rect.width() - total_width) / 2;
                let min_left = mon_rect.left + 4;
                let max_left = (mon_rect.right - total_width - 4).max(min_left);
                left = left.clamp(min_left, max_left);
                (left, top, left + total_width, bottom)
            }
            DockEdge::Left => {
                let left = anchor_rect.right + POPUP_GAP;
                let right = left + total_width;
                let mut top = anchor_rect.top + (anchor_rect.height() - total_height) / 2;
                let min_top = mon_rect.top + 4;
                let max_top = (mon_rect.bottom - total_height - 4).max(min_top);
                top = top.clamp(min_top, max_top);
                (left, top, right, top + total_height)
            }
            DockEdge::Right => {
                let right = anchor_rect.left - POPUP_GAP;
                let left = right - total_width;
                let mut top = anchor_rect.top + (anchor_rect.height() - total_height) / 2;
                let min_top = mon_rect.top + 4;
                let max_top = (mon_rect.bottom - total_height - 4).max(min_top);
                top = top.clamp(min_top, max_top);
                (left, top, right, top + total_height)
            }
        };

        let bounds = BarRect::new(left, top, right, bottom);

        let mut tiles = Vec::with_capacity(task.windows.len());
        for (i, win) in task.windows.iter().enumerate() {
            let tile_left = left + POPUP_PADDING + i as i32 * (TILE_WIDTH + TILE_PADDING);
            let tile_top = top + POPUP_PADDING;
            let tile_right = tile_left + TILE_WIDTH;
            let tile_bottom = tile_top + TILE_HEIGHT;
            let tile_bounds = BarRect::new(tile_left, tile_top, tile_right, tile_bottom);

            let close_btn_right = tile_right - 4;
            let close_btn_left = close_btn_right - CLOSE_BTN_SIZE;
            let close_btn_top = tile_top + 3;
            let close_btn_bottom = close_btn_top + CLOSE_BTN_SIZE;
            let close_button_rect =
                BarRect::new(close_btn_left, close_btn_top, close_btn_right, close_btn_bottom);

            let thumb_left = tile_left + 6;
            let thumb_right = tile_right - 6;
            let thumb_top = tile_top + HEADER_HEIGHT + 2;
            let thumb_bottom = tile_bottom - 6;
            let thumbnail_rect = BarRect::new(thumb_left, thumb_top, thumb_right, thumb_bottom);

            tiles.push(PreviewTile {
                hwnd: win.hwnd,
                title: win.title.clone(),
                bounds: tile_bounds,
                thumbnail_rect,
                close_button_rect,
                is_active: win.is_active,
                is_minimized: win.is_minimized,
            });
        }

        Self {
            hwnd: 0,
            bounds,
            tiles,
            hovered_app_id: task.app_id.clone(),
            hovered_tile_index: None,
            sessions: Vec::new(),
        }
    }

    /// Handles a click event in screen coordinates, mapping it to either a Close or Activate action.
    pub fn handle_click(&self, x: i32, y: i32) -> Option<PreviewClickAction> {
        for tile in &self.tiles {
            if tile.close_button_rect.contains(x, y) {
                return Some(PreviewClickAction::CloseWindow(tile.hwnd));
            } else if tile.bounds.contains(x, y) {
                return Some(PreviewClickAction::ActivateWindow(tile.hwnd));
            }
        }
        None
    }

    /// Updates the hovered tile index based on mouse screen coordinates.
    pub fn handle_mouse_move(&mut self, x: i32, y: i32) -> Option<usize> {
        let found = self.tiles.iter().position(|t| t.bounds.contains(x, y));
        self.hovered_tile_index = found;
        found
    }

    /// Registers live DWM thumbnails for all tiles against the given destination window HWND.
    pub fn attach_thumbnails(&mut self, dest_hwnd: isize) {
        self.clear_thumbnails();
        for tile in &self.tiles {
            let client_rect = tile.client_thumbnail_rect(&self.bounds);
            if let Some(session) = create_dwm_thumbnail(dest_hwnd, tile.hwnd, client_rect) {
                self.sessions.push(session);
            }
        }
    }

    /// Unregisters and drops all active DWM thumbnail sessions.
    pub fn clear_thumbnails(&mut self) {
        self.sessions.clear();
    }

    /// Shows a native Win32 tool window popup for this preview and attaches live DWM thumbnails.
    pub fn show_native_window(&mut self) -> bool {
        if self.hwnd == 0 {
            if let Some(h) = create_preview_window(self.bounds) {
                self.hwnd = h;
                self.attach_thumbnails(h);
                return true;
            }
        }
        false
    }

    /// Hides and destroys the native Win32 window and unregisters all DWM thumbnails.
    pub fn hide_native_window(&mut self) -> bool {
        self.clear_thumbnails();
        if self.hwnd != 0 {
            let ok = destroy_preview_window(self.hwnd);
            self.hwnd = 0;
            return ok;
        }
        true
    }
}

impl Drop for PreviewPopup {
    fn drop(&mut self) {
        let _ = self.hide_native_window();
    }
}

/// Controller managing the hover state, layout calculation, and lifecycle of preview popups.
#[derive(Debug, Default)]
pub struct PreviewPopupController {
    current: Option<PreviewPopup>,
}

impl PreviewPopupController {
    pub fn new() -> Self {
        Self { current: None }
    }

    pub fn is_active(&self) -> bool {
        self.current.is_some()
    }

    pub fn current_popup(&self) -> Option<&PreviewPopup> {
        self.current.as_ref()
    }

    pub fn current_popup_mut(&mut self) -> Option<&mut PreviewPopup> {
        self.current.as_mut()
    }

    pub fn show_for_task(
        &mut self,
        task: &GroupedTask,
        anchor_rect: BarRect,
        edge: DockEdge,
        mon_rect: BarRect,
    ) {
        let popup = PreviewPopup::calculate_layout(task, anchor_rect, edge, mon_rect);
        self.current = Some(popup);
    }

    pub fn hide(&mut self) {
        self.current = None;
    }

    pub fn handle_click(&mut self, x: i32, y: i32) -> Option<PreviewClickAction> {
        let action = self.current.as_ref().and_then(|p| p.handle_click(x, y));
        if let Some(act) = action {
            act.execute();
            if let PreviewClickAction::ActivateWindow(_) = act {
                self.hide();
            }
        }
        action
    }

    pub fn is_hovered(&self, x: i32, y: i32, anchor_rect: BarRect) -> bool {
        if anchor_rect.contains(x, y) {
            return true;
        }
        if let Some(p) = &self.current {
            if p.bounds.contains(x, y) {
                return true;
            }
        }
        false
    }
}
