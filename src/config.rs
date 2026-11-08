use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DockEdge {
    Bottom,
    Top,
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskAlignment {
    Left,
    Center,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemePreset {
    Glossy,
    Material,
    Retro,
    Oled,
    Custom,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PinnedItem {
    pub title: String,
    pub path: String,
    pub args: String,
    pub icon: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    pub edge: DockEdge,
    pub alignment: TaskAlignment,
    pub theme: ThemePreset,
    pub auto_hide: bool,
    pub size: u32,
    pub pinned: Vec<PinnedItem>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            edge: DockEdge::Bottom,
            alignment: TaskAlignment::Center,
            theme: ThemePreset::Glossy,
            auto_hide: false,
            size: 48,
            pinned: Vec::new(),
        }
    }
}

impl Config {
    pub fn load_or_default() -> Self {
        if let Ok(app_data) = std::env::var("APPDATA") {
            let path = std::path::PathBuf::from(app_data).join("taskbar-clone").join("config.toml");
            if let Ok(contents) = std::fs::read_to_string(&path) {
                if let Ok(cfg) = toml::from_str(&contents) {
                    return cfg;
                }
            }
        }
        Config::default()
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        if let Ok(app_data) = std::env::var("APPDATA") {
            let dir = std::path::PathBuf::from(app_data).join("taskbar-clone");
            std::fs::create_dir_all(&dir)?;
            let path = dir.join("config.toml");
            let contents = toml::to_string_pretty(self)
                .map_err(std::io::Error::other)?;
            std::fs::write(path, contents)?;
        }
        Ok(())
    }
}

