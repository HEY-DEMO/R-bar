use std::fs;
use std::path::PathBuf;
use taskbar_clone::config::{Config, DockEdge, TaskAlignment, ThemePreset};

#[test]
fn test_default_config() {
    let cfg = Config::default();
    assert_eq!(cfg.edge, DockEdge::Bottom);
    assert_eq!(cfg.alignment, TaskAlignment::Center);
    assert_eq!(cfg.theme, ThemePreset::Glossy);
    assert!(!cfg.auto_hide);
    assert_eq!(cfg.size, 48);
    assert!(cfg.pinned.is_empty());
    assert!(!cfg.is_vertical());
    assert!(cfg.is_horizontal());
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
    assert!(!cfg.is_vertical());
    assert!(cfg.is_horizontal());
}

#[test]
fn test_config_orientation() {
    let cfg_bottom = Config {
        edge: DockEdge::Bottom,
        ..Default::default()
    };
    assert!(!cfg_bottom.is_vertical());
    assert!(cfg_bottom.is_horizontal());

    let cfg_top = Config {
        edge: DockEdge::Top,
        ..Default::default()
    };
    assert!(!cfg_top.is_vertical());
    assert!(cfg_top.is_horizontal());

    let cfg_left = Config {
        edge: DockEdge::Left,
        ..Default::default()
    };
    assert!(cfg_left.is_vertical());
    assert!(!cfg_left.is_horizontal());

    let cfg_right = Config {
        edge: DockEdge::Right,
        ..Default::default()
    };
    assert!(cfg_right.is_vertical());
    assert!(!cfg_right.is_horizontal());
}

#[test]
fn test_all_dock_edges_and_theme_presets() {
    #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
    struct EdgeWrapper {
        edge: DockEdge,
    }

    #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
    struct ThemeWrapper {
        theme: ThemePreset,
    }

    #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
    struct AlignWrapper {
        alignment: TaskAlignment,
    }

    for edge in [
        DockEdge::Bottom,
        DockEdge::Top,
        DockEdge::Left,
        DockEdge::Right,
    ] {
        let wrapper = EdgeWrapper { edge };
        let serialized = toml::to_string(&wrapper).unwrap();
        let deserialized: EdgeWrapper = toml::from_str(&serialized).unwrap();
        assert_eq!(wrapper, deserialized);
    }

    for theme in [
        ThemePreset::Glossy,
        ThemePreset::Material,
        ThemePreset::Retro,
        ThemePreset::Oled,
        ThemePreset::Custom,
    ] {
        let wrapper = ThemeWrapper { theme };
        let serialized = toml::to_string(&wrapper).unwrap();
        let deserialized: ThemeWrapper = toml::from_str(&serialized).unwrap();
        assert_eq!(wrapper, deserialized);
    }

    for alignment in [TaskAlignment::Left, TaskAlignment::Center] {
        let wrapper = AlignWrapper { alignment };
        let serialized = toml::to_string(&wrapper).unwrap();
        let deserialized: AlignWrapper = toml::from_str(&serialized).unwrap();
        assert_eq!(wrapper, deserialized);
    }
}

#[test]
fn test_default_config_path_resolution() {
    if let Some(path) = Config::default_config_path() {
        assert!(path.ends_with(PathBuf::from("taskbar-clone").join("config.toml")));
    }
}

#[test]
fn test_auto_generates_default_file_if_missing() {
    let temp_dir = std::env::temp_dir().join(format!("taskbar_clone_test_gen_{}", std::process::id()));
    let config_path = temp_dir.join("config.toml");

    let _ = fs::remove_dir_all(&temp_dir);
    assert!(!config_path.exists());

    // Calling load_from_path on a missing path should generate default config.toml on disk
    let loaded = Config::load_from_path(&config_path);
    assert_eq!(loaded, Config::default());
    assert!(config_path.exists(), "config.toml should have been generated");

    // Read the file and verify valid TOML round-trip
    let contents = fs::read_to_string(&config_path).expect("Failed to read generated config");
    let parsed: Config = toml::from_str(&contents).expect("Generated file should be valid TOML");
    assert_eq!(parsed, Config::default());

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_fallback_on_malformed_syntax() {
    let temp_dir = std::env::temp_dir().join(format!("taskbar_clone_test_malformed_{}", std::process::id()));
    fs::create_dir_all(&temp_dir).unwrap();
    let config_path = temp_dir.join("config.toml");

    // Write malformed TOML
    fs::write(&config_path, "[[invalid syntax not a valid TOML document:::").unwrap();

    // Calling load_from_path should not panic or error, but return default config
    let loaded = Config::load_from_path(&config_path);
    assert_eq!(loaded, Config::default());

    let _ = fs::remove_dir_all(&temp_dir);
}
