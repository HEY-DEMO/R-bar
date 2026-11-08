use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DockEdge {
    Bottom,
    Top,
    Left,
    Right,
}

impl DockEdge {
    /// Returns true if the dock edge is positioned along the Left or Right screen boundary (vertical layout).
    pub fn is_vertical(&self) -> bool {
        matches!(self, DockEdge::Left | DockEdge::Right)
    }

    /// Returns true if the dock edge is positioned along the Bottom or Top screen boundary (horizontal layout).
    pub fn is_horizontal(&self) -> bool {
        matches!(self, DockEdge::Bottom | DockEdge::Top)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskAlignment {
    Left,
    Center,
}

pub use crate::theme::ThemePreset;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PinnedItem {
    pub title: String,
    pub path: String,
    pub args: String,
    pub icon: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    /// Returns true if the configured dock edge requires a vertical UI layout orientation.
    pub fn is_vertical(&self) -> bool {
        self.edge.is_vertical()
    }

    /// Returns true if the configured dock edge requires a horizontal UI layout orientation.
    pub fn is_horizontal(&self) -> bool {
        self.edge.is_horizontal()
    }

    /// Resolves the default configuration file path (`%APPDATA%\taskbar-clone\config.toml`).
    pub fn default_config_path() -> Option<PathBuf> {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .map(|base| base.join("taskbar-clone").join("config.toml"))
    }

    /// Loads configuration from the given file path.
    ///
    /// If the file does not exist, automatically generates a default `config.toml` file at that path.
    /// If the file contains malformed syntax or cannot be read, falls back gracefully to `Config::default()`.
    pub fn load_from_path(path: &Path) -> Self {
        if path.exists() {
            match std::fs::read_to_string(path) {
                Ok(contents) => match toml::from_str(&contents) {
                    Ok(cfg) => cfg,
                    Err(err) => {
                        log::warn!(
                            "Failed to parse config at {}: {}. Falling back to default configuration.",
                            path.display(),
                            err
                        );
                        Config::default()
                    }
                },
                Err(err) => {
                    log::warn!(
                        "Failed to read config at {}: {}. Falling back to default configuration.",
                        path.display(),
                        err
                    );
                    Config::default()
                }
            }
        } else {
            let default_cfg = Config::default();
            if let Err(err) = default_cfg.save_to_path(path) {
                log::warn!(
                    "Failed to generate default configuration at {}: {}",
                    path.display(),
                    err
                );
            }
            default_cfg
        }
    }

    /// Loads the configuration from the user's application data directory (`%APPDATA%\taskbar-clone\config.toml`),
    /// auto-generating defaults if missing and falling back gracefully on syntax errors.
    pub fn load_or_default() -> Self {
        if let Some(path) = Self::default_config_path() {
            Self::load_from_path(&path)
        } else {
            Config::default()
        }
    }

    /// Saves the current configuration to the specified file path, ensuring parent directories exist.
    pub fn save_to_path(&self, path: &Path) -> Result<(), std::io::Error> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let contents = toml::to_string_pretty(self).map_err(std::io::Error::other)?;
        std::fs::write(path, contents)
    }

    /// Saves the current configuration to `%APPDATA%\taskbar-clone\config.toml`.
    pub fn save(&self) -> Result<(), std::io::Error> {
        if let Some(path) = Self::default_config_path() {
            self.save_to_path(&path)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "%APPDATA% environment variable not found",
            ))
        }
    }
}
