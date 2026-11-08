use taskbar_clone::shell::{
    get_native_taskbar_classes, hide_native_taskbar, install_panic_watchdog, restore_native_taskbar,
};

#[test]
fn test_native_taskbar_classes() {
    let classes = get_native_taskbar_classes();
    assert_eq!(classes.len(), 2);
    assert!(classes.contains(&"Shell_TrayWnd"));
    assert!(classes.contains(&"Shell_SecondaryTrayWnd"));
}

#[test]
fn test_hide_and_restore_native_taskbar_interface() {
    // Test that hide and restore interfaces execute cleanly and don't panic.
    // We immediately restore to preserve the user's desktop state.
    hide_native_taskbar();
    restore_native_taskbar();
}

#[test]
fn test_install_panic_watchdog() {
    // Installing panic watchdog should succeed and register the hook without error
    install_panic_watchdog();
}

