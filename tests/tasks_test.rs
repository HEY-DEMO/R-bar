use taskbar_clone::events::ShellEvent;
use taskbar_clone::tasks::{
    check_window_qualification_criteria, enumerate_initial_windows, group_tasks,
    handle_window_activated, handle_window_destroyed, handle_window_minimized,
    update_tasks_on_event, GroupedTask, TaskItem, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW,
};

#[test]
fn test_task_item_and_grouping() {
    let items = vec![
        TaskItem {
            hwnd: 101,
            pid: 10,
            title: "Doc 1 - Word".into(),
            app_id: "WINWORD.EXE".into(),
            is_active: false,
            is_minimized: false,
        },
        TaskItem {
            hwnd: 102,
            pid: 10,
            title: "Doc 2 - Word".into(),
            app_id: "WINWORD.EXE".into(),
            is_active: true,
            is_minimized: false,
        },
        TaskItem {
            hwnd: 201,
            pid: 20,
            title: "Terminal".into(),
            app_id: "wt.exe".into(),
            is_active: false,
            is_minimized: false,
        },
    ];

    let groups = group_tasks(items);
    assert_eq!(groups.len(), 2);

    let word_group: &GroupedTask = groups.iter().find(|g| g.app_id == "WINWORD.EXE").unwrap();
    assert_eq!(word_group.windows.len(), 2);
    assert_eq!(word_group.title, "Doc 2 - Word"); // Active window title takes precedence

    let term_group = groups.iter().find(|g| g.app_id == "wt.exe").unwrap();
    assert_eq!(term_group.windows.len(), 1);
    assert_eq!(term_group.title, "Terminal");
}

#[test]
fn test_window_qualification_criteria() {
    // 1. Normal standard application window (visible, no toolwindow, no owner, uncloaked, has title)
    assert!(check_window_qualification_criteria(
        true,
        0,
        0,
        false,
        "Mozilla Firefox"
    ));

    // 2. Invisible window -> fails
    assert!(!check_window_qualification_criteria(
        false,
        0,
        0,
        false,
        "Background Service"
    ));

    // 3. Tool window without WS_EX_APPWINDOW -> fails
    assert!(!check_window_qualification_criteria(
        true,
        WS_EX_TOOLWINDOW,
        0,
        false,
        "Floating Tooltip"
    ));

    // 4. Tool window WITH explicit WS_EX_APPWINDOW -> qualifies
    assert!(check_window_qualification_criteria(
        true,
        WS_EX_TOOLWINDOW | WS_EX_APPWINDOW,
        0,
        false,
        "App Window With Tool Style"
    ));

    // 5. Owned window (GW_OWNER != 0) -> fails
    assert!(!check_window_qualification_criteria(
        true,
        0,
        9999,
        false,
        "Child Dialog"
    ));

    // 6. Cloaked by DWM -> fails
    assert!(!check_window_qualification_criteria(
        true,
        0,
        0,
        true,
        "Cloaked UWP App"
    ));

    // 7. Empty or whitespace-only title -> fails
    assert!(!check_window_qualification_criteria(
        true,
        0,
        0,
        false,
        ""
    ));
    assert!(!check_window_qualification_criteria(
        true,
        0,
        0,
        false,
        "   "
    ));
}

#[test]
fn test_task_lifecycle_tracking() {
    let mut tasks = vec![
        TaskItem {
            hwnd: 100,
            pid: 10,
            title: "Editor".into(),
            app_id: "code.exe".into(),
            is_active: true,
            is_minimized: false,
        },
        TaskItem {
            hwnd: 200,
            pid: 20,
            title: "Browser".into(),
            app_id: "chrome.exe".into(),
            is_active: false,
            is_minimized: false,
        },
    ];

    // Activate window 200: 200 becomes active, 100 becomes inactive
    handle_window_activated(&mut tasks, 200);
    assert!(!tasks[0].is_active);
    assert!(tasks[1].is_active);
    assert!(!tasks[1].is_minimized);

    // Minimize window 200
    handle_window_minimized(&mut tasks, 200, true);
    assert!(!tasks[1].is_active);
    assert!(tasks[1].is_minimized);

    // Destroy window 100: removed immediately
    handle_window_destroyed(&mut tasks, 100);
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].hwnd, 200);

    // Test update_tasks_on_event for Destroyed
    let event = ShellEvent::WindowDestroyed(200);
    update_tasks_on_event(&mut tasks, &event);
    assert!(tasks.is_empty());
}

#[test]
fn test_initial_window_enumeration_smoke() {
    // Should run without crashing and return a list of current OS windows
    let windows = enumerate_initial_windows();
    for window in &windows {
        assert!(window.hwnd != 0);
        assert!(!window.title.is_empty());
    }
}

#[test]
fn test_invalid_hwnd_behavior() {
    // Zero handle should not qualify
    assert!(!taskbar_clone::tasks::is_qualifying_window(0));

    // Calling activate/minimize/close on dummy hwnd 0 should not panic
    taskbar_clone::tasks::activate_window(0);
    taskbar_clone::tasks::minimize_window(0);
    taskbar_clone::tasks::close_window(0);
}
