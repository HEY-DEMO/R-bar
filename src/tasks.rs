use windows::core::PWSTR;
use windows::Win32::Foundation::{CloseHandle, BOOL, HWND, LPARAM, WPARAM};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_FORMAT, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetForegroundWindow, GetWindow, GetWindowLongPtrW, GetWindowTextW,
    GetWindowThreadProcessId, IsIconic, IsWindow, IsWindowVisible, PostMessageW,
    SetForegroundWindow, ShowWindow, GWL_EXSTYLE, GW_OWNER, SC_CLOSE, SW_MINIMIZE, SW_RESTORE,
    WM_SYSCOMMAND,
};

pub const WS_EX_TOOLWINDOW: u32 = 0x00000080;
pub const WS_EX_APPWINDOW: u32 = 0x00040000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskItem {
    pub hwnd: isize,
    pub pid: u32,
    pub title: String,
    pub app_id: String,
    pub is_active: bool,
    pub is_minimized: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupedTask {
    pub app_id: String,
    pub title: String,
    pub windows: Vec<TaskItem>,
}

pub fn check_window_qualification_criteria(
    is_visible: bool,
    ex_style: u32,
    owner: isize,
    is_cloaked: bool,
    title: &str,
) -> bool {
    if !is_visible {
        return false;
    }
    let has_tool_window = (ex_style & WS_EX_TOOLWINDOW) != 0;
    let has_app_window = (ex_style & WS_EX_APPWINDOW) != 0;
    if has_tool_window && !has_app_window {
        return false;
    }
    if owner != 0 {
        return false;
    }
    if is_cloaked {
        return false;
    }
    if title.trim().is_empty() {
        return false;
    }
    true
}

pub fn get_window_title(hwnd: HWND) -> String {
    let mut buffer = [0u16; 512];
    let len = unsafe { GetWindowTextW(hwnd, &mut buffer) };
    if len > 0 {
        String::from_utf16_lossy(&buffer[..len as usize])
    } else {
        String::new()
    }
}

pub fn get_process_image_name(pid: u32) -> Option<String> {
    if pid == 0 {
        return None;
    }
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok()?;
    let mut buffer = [0u16; 1024];
    let mut size = buffer.len() as u32;
    let res = unsafe {
        QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_FORMAT(0),
            PWSTR(buffer.as_mut_ptr()),
            &mut size,
        )
    };
    let _ = unsafe { CloseHandle(handle) };
    if res.is_ok() && size > 0 {
        let full_path = String::from_utf16_lossy(&buffer[..size as usize]);
        let filename = full_path.rsplit(['\\', '/']).next().unwrap_or(&full_path);
        Some(filename.to_string())
    } else {
        None
    }
}

pub fn is_qualifying_window(hwnd: isize) -> bool {
    let h = HWND(hwnd as *mut _);
    if unsafe { !IsWindow(h).as_bool() } {
        return false;
    }
    if unsafe { !IsWindowVisible(h).as_bool() } {
        return false;
    }
    let ex_style = unsafe { GetWindowLongPtrW(h, GWL_EXSTYLE) } as u32;
    let owner = match unsafe { GetWindow(h, GW_OWNER) } {
        Ok(owner_hwnd) => owner_hwnd.0 as isize,
        Err(_) => 0,
    };

    let mut cloaked: u32 = 0;
    let is_cloaked = unsafe {
        DwmGetWindowAttribute(
            h,
            DWMWA_CLOAKED,
            &mut cloaked as *mut _ as *mut std::ffi::c_void,
            std::mem::size_of::<u32>() as u32,
        )
        .is_ok()
            && cloaked != 0
    };

    let title = get_window_title(h);
    check_window_qualification_criteria(true, ex_style, owner, is_cloaked, &title)
}

pub fn create_task_item(hwnd: HWND) -> Option<TaskItem> {
    let mut pid: u32 = 0;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    let title = get_window_title(hwnd);
    let app_id = get_process_image_name(pid).unwrap_or_else(|| {
        if !title.is_empty() {
            title.clone()
        } else {
            format!("pid_{}", pid)
        }
    });

    let foreground = unsafe { GetForegroundWindow() };
    let is_active = foreground.0 == hwnd.0;
    let is_minimized = unsafe { IsIconic(hwnd).as_bool() };

    Some(TaskItem {
        hwnd: hwnd.0 as isize,
        pid,
        title,
        app_id,
        is_active,
        is_minimized,
    })
}

pub fn group_tasks(tasks: Vec<TaskItem>) -> Vec<GroupedTask> {
    let mut groups: Vec<GroupedTask> = Vec::new();
    for item in tasks {
        if let Some(existing) = groups.iter_mut().find(|g| g.app_id == item.app_id) {
            if item.is_active {
                existing.title = item.title.clone();
            }
            existing.windows.push(item);
        } else {
            groups.push(GroupedTask {
                app_id: item.app_id.clone(),
                title: item.title.clone(),
                windows: vec![item],
            });
        }
    }
    groups
}

pub fn activate_window(hwnd: isize) {
    let h = HWND(hwnd as *mut _);
    if unsafe { IsIconic(h).as_bool() } {
        let _ = unsafe { ShowWindow(h, SW_RESTORE) };
    }
    let _ = unsafe { SetForegroundWindow(h) };
}

pub fn minimize_window(hwnd: isize) {
    let h = HWND(hwnd as *mut _);
    let _ = unsafe { ShowWindow(h, SW_MINIMIZE) };
}

pub fn close_window(hwnd: isize) {
    let h = HWND(hwnd as *mut _);
    let _ = unsafe { PostMessageW(h, WM_SYSCOMMAND, WPARAM(SC_CLOSE as usize), LPARAM(0)) };
}

pub fn handle_window_activated(tasks: &mut [TaskItem], active_hwnd: isize) {
    for item in tasks.iter_mut() {
        if item.hwnd == active_hwnd {
            item.is_active = true;
            item.is_minimized = false;
        } else {
            item.is_active = false;
        }
    }
}

pub fn handle_window_minimized(tasks: &mut [TaskItem], hwnd: isize, is_minimized: bool) {
    for item in tasks.iter_mut() {
        if item.hwnd == hwnd {
            item.is_minimized = is_minimized;
            if is_minimized {
                item.is_active = false;
            }
        }
    }
}

pub fn handle_window_destroyed(tasks: &mut Vec<TaskItem>, hwnd: isize) {
    tasks.retain(|t| t.hwnd != hwnd);
}

pub fn enumerate_initial_windows() -> Vec<TaskItem> {
    let mut tasks = Vec::new();

    unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let tasks_ptr = lparam.0 as *mut Vec<TaskItem>;
        let hwnd_isize = hwnd.0 as isize;
        if is_qualifying_window(hwnd_isize) {
            if let Some(task) = create_task_item(hwnd) {
                unsafe {
                    (*tasks_ptr).push(task);
                }
            }
        }
        BOOL(1)
    }

    let lparam = LPARAM(&mut tasks as *mut _ as isize);
    let _ = unsafe { EnumWindows(Some(enum_proc), lparam) };
    tasks
}

pub fn update_tasks_on_event(tasks: &mut Vec<TaskItem>, event: &crate::events::ShellEvent) {
    match event {
        crate::events::ShellEvent::WindowCreated(hwnd) => {
            if is_qualifying_window(*hwnd) && !tasks.iter().any(|t| t.hwnd == *hwnd) {
                let h = HWND(*hwnd as *mut _);
                if let Some(item) = create_task_item(h) {
                    tasks.push(item);
                }
            }
        }
        crate::events::ShellEvent::WindowDestroyed(hwnd) => {
            handle_window_destroyed(tasks, *hwnd);
        }
        crate::events::ShellEvent::WindowActivated(hwnd) => {
            handle_window_activated(tasks, *hwnd);
        }
        crate::events::ShellEvent::WindowRedraw(hwnd) => {
            if let Some(item) = tasks.iter_mut().find(|t| t.hwnd == *hwnd) {
                let h = HWND(*hwnd as *mut _);
                item.title = get_window_title(h);
                item.is_minimized = unsafe { IsIconic(h).as_bool() };
            }
        }
        _ => {}
    }
}
