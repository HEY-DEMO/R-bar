use taskbar_clone::controls::{
    execute_window_command, handle_task_click, handle_task_click_state,
    TaskClickAction, WindowCommand,
};
use taskbar_clone::tasks::{group_tasks, GroupedTask, TaskItem};

#[test]
fn test_grouped_task_window_count_and_indicators() {
    let single_item = TaskItem {
        hwnd: 101,
        pid: 10,
        title: "Notepad".into(),
        app_id: "notepad.exe".into(),
        is_active: false,
        is_minimized: false,
    };
    let single_group = GroupedTask {
        app_id: "notepad.exe".into(),
        title: "Notepad".into(),
        windows: vec![single_item],
    };

    assert_eq!(single_group.window_count(), 1);
    assert!(!single_group.has_multiple_windows());
    assert!(!single_group.is_active());
    assert!(single_group.active_window().is_none());
    assert_eq!(single_group.primary_window().unwrap().hwnd, 101);

    let multi_group = GroupedTask {
        app_id: "chrome.exe".into(),
        title: "Tab 2 - Chrome".into(),
        windows: vec![
            TaskItem {
                hwnd: 201,
                pid: 20,
                title: "Tab 1 - Chrome".into(),
                app_id: "chrome.exe".into(),
                is_active: false,
                is_minimized: true,
            },
            TaskItem {
                hwnd: 202,
                pid: 20,
                title: "Tab 2 - Chrome".into(),
                app_id: "chrome.exe".into(),
                is_active: true,
                is_minimized: false,
            },
            TaskItem {
                hwnd: 203,
                pid: 20,
                title: "Tab 3 - Chrome".into(),
                app_id: "chrome.exe".into(),
                is_active: false,
                is_minimized: false,
            },
        ],
    };

    assert_eq!(multi_group.window_count(), 3);
    assert!(multi_group.has_multiple_windows());
    assert!(multi_group.is_active());
    assert_eq!(multi_group.active_window().unwrap().hwnd, 202);
    assert_eq!(multi_group.primary_window().unwrap().hwnd, 202);
}

#[test]
fn test_task_click_toggle_behavior() {
    // 1. Inactive group: clicking should activate primary window
    let inactive_group = GroupedTask {
        app_id: "calc.exe".into(),
        title: "Calculator".into(),
        windows: vec![TaskItem {
            hwnd: 301,
            pid: 30,
            title: "Calculator".into(),
            app_id: "calc.exe".into(),
            is_active: false,
            is_minimized: true,
        }],
    };

    let action = handle_task_click(&inactive_group);
    assert_eq!(action, TaskClickAction::Activated(301));

    // 2. Active group: clicking should minimize the active window
    let active_group = GroupedTask {
        app_id: "calc.exe".into(),
        title: "Calculator".into(),
        windows: vec![TaskItem {
            hwnd: 301,
            pid: 30,
            title: "Calculator".into(),
            app_id: "calc.exe".into(),
            is_active: true,
            is_minimized: false,
        }],
    };

    let action_active = handle_task_click(&active_group);
    assert_eq!(action_active, TaskClickAction::Minimized(301));
}

#[test]
fn test_task_click_state_mutation() {
    let mut tasks = vec![
        TaskItem {
            hwnd: 1001,
            pid: 10,
            title: "Doc 1".into(),
            app_id: "app.exe".into(),
            is_active: false,
            is_minimized: false,
        },
        TaskItem {
            hwnd: 1002,
            pid: 10,
            title: "Doc 2".into(),
            app_id: "app.exe".into(),
            is_active: false,
            is_minimized: false,
        },
    ];

    // Click on 1001: should activate
    let act1 = handle_task_click_state(&mut tasks, 1001);
    assert_eq!(act1, TaskClickAction::Activated(1001));
    assert!(tasks[0].is_active);
    assert!(!tasks[0].is_minimized);
    assert!(!tasks[1].is_active);

    // Click on 1001 again: should toggle to minimized
    let act2 = handle_task_click_state(&mut tasks, 1001);
    assert_eq!(act2, TaskClickAction::Minimized(1001));
    assert!(!tasks[0].is_active);
    assert!(tasks[0].is_minimized);

    // Click on 1002: should activate 1002, 1001 remains inactive
    let act3 = handle_task_click_state(&mut tasks, 1002);
    assert_eq!(act3, TaskClickAction::Activated(1002));
    assert!(!tasks[0].is_active);
    assert!(tasks[1].is_active);
    assert!(!tasks[1].is_minimized);

    // Click on non-existent window: returns None
    let act4 = handle_task_click_state(&mut tasks, 9999);
    assert_eq!(act4, TaskClickAction::None);
}

#[test]
fn test_window_commands_and_syscommand_codes() {
    assert_eq!(WindowCommand::Restore.syscommand_code(), 0xF120);
    assert_eq!(WindowCommand::Minimize.syscommand_code(), 0xF020);
    assert_eq!(WindowCommand::Maximize.syscommand_code(), 0xF030);
    assert_eq!(WindowCommand::Close.syscommand_code(), 0xF060);

    // Invoking commands on dummy/test handles does not crash or panic
    assert!(execute_window_command(0, WindowCommand::Restore));
    assert!(execute_window_command(0, WindowCommand::Minimize));
    assert!(execute_window_command(0, WindowCommand::Maximize));
    assert!(execute_window_command(0, WindowCommand::Close));
}

#[test]
fn test_multi_window_grouping_clusters_and_controls() {
    let items = vec![
        TaskItem {
            hwnd: 501,
            pid: 50,
            title: "Project A - VSCode".into(),
            app_id: "code.exe".into(),
            is_active: false,
            is_minimized: false,
        },
        TaskItem {
            hwnd: 502,
            pid: 50,
            title: "Project B - VSCode".into(),
            app_id: "code.exe".into(),
            is_active: true,
            is_minimized: false,
        },
    ];

    let groups = group_tasks(items);
    assert_eq!(groups.len(), 1);
    let group = &groups[0];

    assert_eq!(group.window_count(), 2);
    assert!(group.has_multiple_windows());
    assert!(group.is_active());
    assert_eq!(group.title, "Project B - VSCode");

    // Clicking group with active window 502 should minimize 502
    let act = handle_task_click(group);
    assert_eq!(act, TaskClickAction::Minimized(502));
}
