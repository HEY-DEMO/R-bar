use crossbeam_channel::unbounded;
use taskbar_clone::events::{
    parse_shell_hook_event, start_event_pump, stop_event_pump, ShellEvent,
    HSHELL_ACTIVATESHELLWINDOW, HSHELL_REDRAW, HSHELL_WINDOWCREATED, HSHELL_WINDOWDESTROYED,
};

#[test]
fn test_shell_event_variants() {
    let events = [
        ShellEvent::WindowCreated(101),
        ShellEvent::WindowDestroyed(101),
        ShellEvent::WindowActivated(101),
        ShellEvent::WindowRedraw(101),
        ShellEvent::DisplayChanged,
        ShellEvent::DpiChanged(144),
        ShellEvent::FullscreenChanged(true),
        ShellEvent::FullscreenChanged(false),
    ];

    assert_eq!(events.len(), 8);

    // Test pattern matching
    match events[0] {
        ShellEvent::WindowCreated(hwnd) => assert_eq!(hwnd, 101),
        _ => panic!("Expected WindowCreated"),
    }

    match events[2] {
        ShellEvent::WindowActivated(hwnd) => assert_eq!(hwnd, 101),
        _ => panic!("Expected WindowActivated"),
    }

    match events[6] {
        ShellEvent::FullscreenChanged(fs) => assert!(fs),
        _ => panic!("Expected FullscreenChanged"),
    }
}

#[test]
fn test_shell_event_channel_dispatch() {
    let (tx, rx) = unbounded();

    tx.send(ShellEvent::WindowCreated(555)).unwrap();
    tx.send(ShellEvent::WindowActivated(555)).unwrap();
    tx.send(ShellEvent::FullscreenChanged(true)).unwrap();

    assert_eq!(rx.recv().unwrap(), ShellEvent::WindowCreated(555));
    assert_eq!(rx.recv().unwrap(), ShellEvent::WindowActivated(555));
    assert_eq!(rx.recv().unwrap(), ShellEvent::FullscreenChanged(true));
}

#[test]
fn test_shell_hook_message_parser() {
    assert_eq!(
        parse_shell_hook_event(HSHELL_WINDOWCREATED as usize, 100),
        Some(ShellEvent::WindowCreated(100))
    );
    assert_eq!(
        parse_shell_hook_event(HSHELL_WINDOWDESTROYED as usize, 100),
        Some(ShellEvent::WindowDestroyed(100))
    );
    assert_eq!(
        parse_shell_hook_event(HSHELL_ACTIVATESHELLWINDOW as usize, 100),
        Some(ShellEvent::WindowActivated(100))
    );
    assert_eq!(
        parse_shell_hook_event(HSHELL_REDRAW as usize, 100),
        Some(ShellEvent::WindowRedraw(100))
    );
    assert_eq!(parse_shell_hook_event(99999, 100), None);
}

#[test]
fn test_start_and_stop_event_pump() {
    let (tx, _rx) = unbounded();
    let handle = start_event_pump(tx);

    // Give the thread a short moment to create the hidden window and register hooks
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Signal stop via stop_event_pump()
    stop_event_pump();

    // The thread handle should join successfully within 1 second
    let join_res = handle.join();
    assert!(join_res.is_ok());
}
