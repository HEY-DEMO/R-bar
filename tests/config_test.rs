use taskbar_clone::config::{Config, DockEdge, TaskAlignment, ThemePreset};

#[test]
fn test_default_config() {
    let cfg = Config::default();
    assert_eq!(cfg.edge, DockEdge::Bottom);
    assert_eq!(cfg.alignment, TaskAlignment::Center);
    assert_eq!(cfg.theme, ThemePreset::Glossy);
    assert!(!cfg.auto_hide);
    assert_eq!(cfg.size, 48);
}

#[test]
fn test_toml_roundtrip() {
    let toml_str = r#"
        edge = "Top"
        alignment = "Left"
        theme = "Material"
        auto_hide = true
        size = 52
        pinned = [
            { title = "Terminal", path = "wt.exe", args = "", icon = "" }
        ]
    "#;
    let cfg: Config = toml::from_str(toml_str).expect("Failed to deserialize TOML");
    assert_eq!(cfg.edge, DockEdge::Top);
    assert_eq!(cfg.alignment, TaskAlignment::Left);
    assert_eq!(cfg.theme, ThemePreset::Material);
    assert!(cfg.auto_hide);
    assert_eq!(cfg.size, 52);
    assert_eq!(cfg.pinned.len(), 1);
    assert_eq!(cfg.pinned[0].title, "Terminal");
}

