use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    GetSystemMenu, PostMessageW, ShowWindow, TrackPopupMenu, SC_CLOSE, SC_MAXIMIZE, SC_MINIMIZE,
    SC_RESTORE, SW_MAXIMIZE, SW_MINIMIZE, SW_RESTORE, TPM_LEFTBUTTON, TPM_RETURNCMD,
    TPM_RIGHTBUTTON, WM_SYSCOMMAND,
};

use crate::tasks::{activate_window, minimize_window, GroupedTask, TaskItem};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowCommand {
    Restore,
    Minimize,
    Maximize,
    Close,
}

impl WindowCommand {
    pub const fn syscommand_code(&self) -> usize {
        match self {
            Self::Restore => SC_RESTORE as usize,
            Self::Minimize => SC_MINIMIZE as usize,
            Self::Maximize => SC_MAXIMIZE as usize,
            Self::Close => SC_CLOSE as usize,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskClickAction {
    Activated(isize),
    Minimized(isize),
    None,
}

/// Handles a taskbar click on a grouped task.
///
/// If any window in the group is currently active, clicking minimizes that active window.
/// If no window in the group is active, clicking brings the primary window to the foreground
/// and restores it if it was minimized.
pub fn handle_task_click(task: &GroupedTask) -> TaskClickAction {
    if let Some(active) = task.active_window() {
        minimize_window(active.hwnd);
        TaskClickAction::Minimized(active.hwnd)
    } else if let Some(target) = task.primary_window() {
        activate_window(target.hwnd);
        TaskClickAction::Activated(target.hwnd)
    } else {
        TaskClickAction::None
    }
}

/// Toggles activation/minimization state for a specific window in a task list.
pub fn handle_task_click_state(tasks: &mut [TaskItem], hwnd: isize) -> TaskClickAction {
    if let Some(idx) = tasks.iter().position(|t| t.hwnd == hwnd) {
        if tasks[idx].is_active {
            minimize_window(hwnd);
            tasks[idx].is_active = false;
            tasks[idx].is_minimized = true;
            TaskClickAction::Minimized(hwnd)
        } else {
            activate_window(hwnd);
            for t in tasks.iter_mut() {
                t.is_active = false;
            }
            tasks[idx].is_active = true;
            tasks[idx].is_minimized = false;
            TaskClickAction::Activated(hwnd)
        }
    } else {
        TaskClickAction::None
    }
}

/// Executes standard window commands (Minimize, Maximize, Restore, Close)
/// via Win32 ShowWindow and WM_SYSCOMMAND messages.
pub fn execute_window_command(hwnd: isize, cmd: WindowCommand) -> bool {
    if hwnd == 0 {
        return true;
    }
    let h = HWND(hwnd as *mut _);
    match cmd {
        WindowCommand::Restore => {
            let _ = unsafe { ShowWindow(h, SW_RESTORE) };
        }
        WindowCommand::Minimize => {
            let _ = unsafe { ShowWindow(h, SW_MINIMIZE) };
        }
        WindowCommand::Maximize => {
            let _ = unsafe { ShowWindow(h, SW_MAXIMIZE) };
        }
        WindowCommand::Close => {}
    }
    let _ = unsafe {
        PostMessageW(
            h,
            WM_SYSCOMMAND,
            WPARAM(cmd.syscommand_code()),
            LPARAM(0),
        )
    };
    true
}

/// Displays the native Win32 window system menu at (x, y) coordinates and
/// executes any command selected by the user.
pub fn open_window_system_menu(hwnd: isize, x: i32, y: i32) -> Option<u32> {
    if hwnd == 0 {
        return None;
    }
    let h = HWND(hwnd as *mut _);
    let hmenu = unsafe { GetSystemMenu(h, false) };
    if hmenu.0.is_null() {
        return None;
    }
    let cmd = unsafe {
        TrackPopupMenu(
            hmenu,
            TPM_LEFTBUTTON | TPM_RIGHTBUTTON | TPM_RETURNCMD,
            x,
            y,
            0,
            h,
            None,
        )
    };
    if cmd.0 != 0 {
        let _ = unsafe {
            PostMessageW(
                h,
                WM_SYSCOMMAND,
                WPARAM(cmd.0 as usize),
                LPARAM(0),
            )
        };
        Some(cmd.0 as u32)
    } else {
        None
    }
}
