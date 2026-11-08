use std::env;
use std::fs;
use taskbar_clone::config::PinnedItem;
use taskbar_clone::launcher::{build_command, launch_app, parse_args, trigger_start_menu};

#[test]
fn test_parse_args_empty_and_whitespace() {
    assert_eq!(parse_args(""), Vec::<String>::new());
    assert_eq!(parse_args("   "), Vec::<String>::new());
}

#[test]
fn test_parse_args_simple_and_quoted() {
    let args = parse_args("/c exit 0");
    assert_eq!(args, vec!["/c", "exit", "0"]);

    let quoted_args = parse_args(r#"--title "My App" --dir 'C:\Program Files\App' --flag"#);
    assert_eq!(
        quoted_args,
        vec!["--title", "My App", "--dir", r"C:\Program Files\App", "--flag"]
    );
}

#[test]
fn test_build_command_structure() {
    let temp_dir = env::temp_dir();
    let temp_str = temp_dir.to_string_lossy().to_string();

    let item = PinnedItem {
        title: "Test Cmd".into(),
        path: "cmd.exe".into(),
        args: "/c exit 0".into(),
        icon: "".into(),
        working_dir: Some(temp_str.clone()),
    };

    let cmd = build_command(&item);
    let debug_str = format!("{:?}", cmd);
    assert!(debug_str.contains("cmd.exe"));
    assert!(debug_str.contains("/c"));
    assert!(debug_str.contains("exit"));
    assert!(debug_str.contains("0"));
}

#[test]
fn test_launch_app_success() {
    let item = PinnedItem {
        title: "Cmd Exit Success".into(),
        path: "cmd.exe".into(),
        args: "/c exit 0".into(),
        icon: "".into(),
        working_dir: None,
    };

    let result = launch_app(&item);
    assert!(result.is_ok(), "Expected cmd.exe /c exit 0 to launch successfully: {:?}", result.err());
}

#[test]
fn test_launch_app_with_working_directory() {
    let temp_dir = env::temp_dir().join(format!("taskbar_launcher_test_{}", std::process::id()));
    fs::create_dir_all(&temp_dir).expect("Failed to create test temp dir");

    let item = PinnedItem {
        title: "Cmd in Temp Dir".into(),
        path: "cmd.exe".into(),
        args: "/c exit 0".into(),
        icon: "".into(),
        working_dir: Some(temp_dir.to_string_lossy().to_string()),
    };

    let result = launch_app(&item);
    assert!(result.is_ok(), "Failed to launch with working_dir: {:?}", result.err());

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_launch_app_missing_executable() {
    let item = PinnedItem {
        title: "Missing Binary".into(),
        path: "non_existent_binary_xyz_987654321.exe".into(),
        args: "".into(),
        icon: "".into(),
        working_dir: None,
    };

    let result = launch_app(&item);
    assert!(result.is_err(), "Expected error launching nonexistent binary");
    let err = result.unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
}

#[test]
fn test_pinned_item_deserialization_with_working_directory_and_aliases() {
    let toml_str = r#"
        title = "File Manager"
        path = "explorer.exe"
        args = "."
        icon = "folder.ico"
        working_dir = "C:\\Windows"
    "#;
    let item: PinnedItem = toml::from_str(toml_str).expect("Failed to deserialize PinnedItem");
    assert_eq!(item.title, "File Manager");
    assert_eq!(item.path, "explorer.exe");
    assert_eq!(item.args, ".");
    assert_eq!(item.icon, "folder.ico");
    assert_eq!(item.working_dir.as_deref(), Some("C:\\Windows"));

    // Also support 'name' and 'working_directory' aliases
    let alias_toml = r#"
        name = "Terminal"
        path = "wt.exe"
        working_directory = "C:\\Users"
    "#;
    let alias_item: PinnedItem = toml::from_str(alias_toml).expect("Failed to deserialize with aliases");
    assert_eq!(alias_item.title, "Terminal");
    assert_eq!(alias_item.path, "wt.exe");
    assert_eq!(alias_item.args, "");
    assert_eq!(alias_item.icon, "");
    assert_eq!(alias_item.working_dir.as_deref(), Some("C:\\Users"));
}

#[test]
fn test_trigger_start_menu_does_not_panic() {
    // Calling trigger_start_menu should synthesize Win key without panic
    trigger_start_menu();
}
