use serde::de::{self, Deserializer, MapAccess, Visitor};
use serde::ser::{SerializeMap, Serializer};
use serde::{Deserialize, Serialize};
use std::fmt;
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Dwm::{
    DwmSetWindowAttribute, DWMWA_SYSTEMBACKDROP_TYPE, DWM_SYSTEMBACKDROP_TYPE,
    DWMSBT_MAINWINDOW, DWMSBT_NONE, DWMSBT_TABBEDWINDOW, DWMSBT_TRANSIENTWINDOW,
};

pub fn default_custom_bg() -> String {
    "#1e1e1ecc".to_string()
}

pub fn default_custom_opacity() -> f32 {
    0.8
}

pub fn default_custom_border() -> String {
    "#ffffff33".to_string()
}

/// Visual theme presets supported by Taskbar Clone.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum ThemePreset {
    #[default]
    Glossy,
    Material,
    Retro,
    Oled,
    Custom {
        bg_color: String,
        opacity: f32,
        border_color: String,
    },
}

impl ThemePreset {
    /// Constructs a custom theme preset with the specified background color, opacity, and border color.
    pub fn custom(bg_color: impl Into<String>, opacity: f32, border_color: impl Into<String>) -> Self {
        Self::Custom {
            bg_color: bg_color.into(),
            opacity,
            border_color: border_color.into(),
        }
    }

    /// Constructs a custom theme preset with default values.
    pub fn default_custom() -> Self {
        Self::Custom {
            bg_color: default_custom_bg(),
            opacity: default_custom_opacity(),
            border_color: default_custom_border(),
        }
    }

    /// Returns the canonical display name of the theme preset.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Glossy => "Glossy",
            Self::Material => "Material",
            Self::Retro => "Retro",
            Self::Oled => "Oled",
            Self::Custom { .. } => "Custom",
        }
    }
}

impl Serialize for ThemePreset {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            ThemePreset::Glossy => serializer.serialize_str("Glossy"),
            ThemePreset::Material => serializer.serialize_str("Material"),
            ThemePreset::Retro => serializer.serialize_str("Retro"),
            ThemePreset::Oled => serializer.serialize_str("Oled"),
            ThemePreset::Custom {
                bg_color,
                opacity,
                border_color,
            } => {
                #[derive(Serialize)]
                struct CustomFields<'a> {
                    bg_color: &'a str,
                    opacity: f32,
                    border_color: &'a str,
                }
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry(
                    "Custom",
                    &CustomFields {
                        bg_color,
                        opacity: *opacity,
                        border_color,
                    },
                )?;
                map.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for ThemePreset {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ThemePresetVisitor;

        impl<'de> Visitor<'de> for ThemePresetVisitor {
            type Value = ThemePreset;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str(
                    "a theme name string ('Glossy', 'Material', 'Retro', 'Oled', 'Custom') or a table mapping 'Custom' to its properties",
                )
            }

            fn visit_str<E>(self, value: &str) -> Result<ThemePreset, E>
            where
                E: de::Error,
            {
                match value.to_lowercase().as_str() {
                    "glossy" => Ok(ThemePreset::Glossy),
                    "material" => Ok(ThemePreset::Material),
                    "retro" => Ok(ThemePreset::Retro),
                    "oled" => Ok(ThemePreset::Oled),
                    "custom" => Ok(ThemePreset::default_custom()),
                    _ => Err(de::Error::unknown_variant(
                        value,
                        &["Glossy", "Material", "Retro", "Oled", "Custom"],
                    )),
                }
            }

            fn visit_map<M>(self, mut access: M) -> Result<ThemePreset, M::Error>
            where
                M: MapAccess<'de>,
            {
                #[derive(Deserialize)]
                struct CustomFields {
                    #[serde(default = "default_custom_bg")]
                    bg_color: String,
                    #[serde(default = "default_custom_opacity")]
                    opacity: f32,
                    #[serde(default = "default_custom_border")]
                    border_color: String,
                }

                while let Some(key) = access.next_key::<String>()? {
                    if key.eq_ignore_ascii_case("custom") {
                        let fields: CustomFields = access.next_value()?;
                        return Ok(ThemePreset::Custom {
                            bg_color: fields.bg_color,
                            opacity: fields.opacity,
                            border_color: fields.border_color,
                        });
                    } else if key.eq_ignore_ascii_case("type") {
                        let variant_name: String = access.next_value()?;
                        return self.visit_str(&variant_name);
                    } else {
                        let _ = access.next_value::<de::IgnoredAny>()?;
                    }
                }

                Ok(ThemePreset::default_custom())
            }
        }

        deserializer.deserialize_any(ThemePresetVisitor)
    }
}

/// DWM system backdrop types supported by Windows 11.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DwmBackdrop {
    None,
    Mica,
    Acrylic,
    MicaAlt,
}

impl DwmBackdrop {
    /// Determines the appropriate DWM backdrop material for a given theme preset.
    pub fn from_preset(preset: &ThemePreset) -> Self {
        match preset {
            ThemePreset::Glossy => DwmBackdrop::Acrylic,
            ThemePreset::Material => DwmBackdrop::None,
            ThemePreset::Retro => DwmBackdrop::None,
            ThemePreset::Oled => DwmBackdrop::None,
            ThemePreset::Custom { opacity, .. } => {
                if *opacity < 1.0 {
                    DwmBackdrop::Acrylic
                } else {
                    DwmBackdrop::None
                }
            }
        }
    }

    /// Converts to the Windows 11 `DWM_SYSTEMBACKDROP_TYPE` integer value.
    pub fn as_dwm_system_backdrop_type(&self) -> u32 {
        match self {
            DwmBackdrop::None => 1,     // DWMSBT_NONE
            DwmBackdrop::Mica => 2,     // DWMSBT_MAINWINDOW
            DwmBackdrop::Acrylic => 3,  // DWMSBT_TRANSIENTWINDOW
            DwmBackdrop::MicaAlt => 4,  // DWMSBT_TABBEDWINDOW
        }
    }

    /// Converts to the Win32 `DWM_SYSTEMBACKDROP_TYPE` representation.
    pub fn as_win32(&self) -> DWM_SYSTEMBACKDROP_TYPE {
        match self {
            DwmBackdrop::None => DWMSBT_NONE,
            DwmBackdrop::Mica => DWMSBT_MAINWINDOW,
            DwmBackdrop::Acrylic => DWMSBT_TRANSIENTWINDOW,
            DwmBackdrop::MicaAlt => DWMSBT_TABBEDWINDOW,
        }
    }
}

/// Fully resolved theme definition containing all visual parameters and DWM configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct ThemeDefinition {
    pub name: String,
    pub bg_color: String,
    pub border_color: String,
    pub border_width: f32,
    pub corner_radius: f32,
    pub accent_color: String,
    pub text_color: String,
    pub opacity: f32,
    pub is_retro_beveled: bool,
    pub dwm_backdrop: DwmBackdrop,
}

impl ThemeDefinition {
    /// Resolves complete styling and DWM backdrop configuration from a theme preset.
    pub fn from_preset(preset: &ThemePreset) -> Self {
        match preset {
            ThemePreset::Glossy => Self {
                name: "Glossy".to_string(),
                bg_color: "#1e1e1e99".to_string(),
                border_color: "#ffffff33".to_string(),
                border_width: 1.0,
                corner_radius: 8.0,
                accent_color: "#60cdff".to_string(),
                text_color: "#ffffff".to_string(),
                opacity: 0.6,
                is_retro_beveled: false,
                dwm_backdrop: DwmBackdrop::Acrylic,
            },
            ThemePreset::Material => Self {
                name: "Material".to_string(),
                bg_color: "#242424".to_string(),
                border_color: "#333333".to_string(),
                border_width: 1.0,
                corner_radius: 4.0,
                accent_color: "#0078d4".to_string(),
                text_color: "#ffffff".to_string(),
                opacity: 1.0,
                is_retro_beveled: false,
                dwm_backdrop: DwmBackdrop::None,
            },
            ThemePreset::Retro => Self {
                name: "Retro".to_string(),
                bg_color: "#c0c0c0".to_string(),
                border_color: "#dfdfdf".to_string(),
                border_width: 2.0,
                corner_radius: 0.0,
                accent_color: "#000080".to_string(),
                text_color: "#000000".to_string(),
                opacity: 1.0,
                is_retro_beveled: true,
                dwm_backdrop: DwmBackdrop::None,
            },
            ThemePreset::Oled => Self {
                name: "Oled".to_string(),
                bg_color: "#000000".to_string(),
                border_color: "#222222".to_string(),
                border_width: 1.0,
                corner_radius: 0.0,
                accent_color: "#ffffff".to_string(),
                text_color: "#ffffff".to_string(),
                opacity: 1.0,
                is_retro_beveled: false,
                dwm_backdrop: DwmBackdrop::None,
            },
            ThemePreset::Custom {
                bg_color,
                opacity,
                border_color,
            } => {
                let opacity = opacity.clamp(0.0, 1.0);
                let dwm_backdrop = if opacity < 1.0 {
                    DwmBackdrop::Acrylic
                } else {
                    DwmBackdrop::None
                };
                Self {
                    name: "Custom".to_string(),
                    bg_color: bg_color.clone(),
                    border_color: border_color.clone(),
                    border_width: 1.0,
                    corner_radius: 4.0,
                    accent_color: "#0078d4".to_string(),
                    text_color: "#ffffff".to_string(),
                    opacity,
                    is_retro_beveled: false,
                    dwm_backdrop,
                }
            }
        }
    }
}

/// Compact representation of theme properties ready for binding into Slint UI properties.
#[derive(Debug, Clone, PartialEq)]
pub struct ThemeStyle {
    pub theme_name: String,
    pub bg_color: String,
    pub border_color: String,
    pub border_width: f32,
    pub corner_radius: f32,
    pub accent_color: String,
    pub text_color: String,
    pub opacity: f32,
    pub is_retro: bool,
}

impl ThemeStyle {
    /// Extracts Slint-ready styling properties from a theme preset.
    pub fn from_preset(preset: &ThemePreset) -> Self {
        let def = ThemeDefinition::from_preset(preset);
        Self {
            theme_name: def.name,
            bg_color: def.bg_color,
            border_color: def.border_color,
            border_width: def.border_width,
            corner_radius: def.corner_radius,
            accent_color: def.accent_color,
            text_color: def.text_color,
            opacity: def.opacity,
            is_retro: def.is_retro_beveled,
        }
    }
}

/// Applies Windows 11 Desktop Window Manager (DWM) backdrop material (Acrylic/Mica) to a window HWND.
///
/// Returns `Ok(())` on success, or `Err(hresult)` if the Win32 call fails or if the HWND is 0/invalid.
pub fn apply_dwm_backdrop(hwnd: isize, backdrop: DwmBackdrop) -> Result<(), isize> {
    if hwnd == 0 {
        return Err(-1);
    }
    unsafe {
        let raw_hwnd = HWND(hwnd as _);
        let backdrop_type = backdrop.as_win32();
        let pv_attribute = &backdrop_type as *const _ as *const std::ffi::c_void;
        let cb_attribute = std::mem::size_of::<DWM_SYSTEMBACKDROP_TYPE>() as u32;

        match DwmSetWindowAttribute(
            raw_hwnd,
            DWMWA_SYSTEMBACKDROP_TYPE,
            pv_attribute,
            cb_attribute,
        ) {
            Ok(()) => Ok(()),
            Err(err) => Err(err.code().0 as isize),
        }
    }
}
