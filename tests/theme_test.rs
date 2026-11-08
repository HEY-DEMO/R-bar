use taskbar_clone::theme::{
    apply_dwm_backdrop, DwmBackdrop, ThemeDefinition, ThemePreset, ThemeStyle,
};

#[test]
fn test_glossy_theme_properties() {
    let preset = ThemePreset::Glossy;
    let def = ThemeDefinition::from_preset(&preset);
    assert_eq!(def.name, "Glossy");
    assert_eq!(def.dwm_backdrop, DwmBackdrop::Acrylic);
    assert!(def.opacity < 1.0, "Glossy should be translucent");
    assert_eq!(def.opacity, 0.6);
    assert!(
        def.bg_color.starts_with("#1e1e1e"),
        "Glossy background should match translucent dark surface"
    );
    assert_eq!(def.border_color, "#ffffff33", "Glossy should have subtle highlight border");
    assert_eq!(def.border_width, 1.0);
    assert_eq!(def.corner_radius, 8.0);
}

#[test]
fn test_material_theme_properties() {
    let preset = ThemePreset::Material;
    let def = ThemeDefinition::from_preset(&preset);
    assert_eq!(def.name, "Material");
    assert_eq!(def.dwm_backdrop, DwmBackdrop::None);
    assert_eq!(def.opacity, 1.0, "Material should have clean, solid surface");
    assert_eq!(def.bg_color, "#242424");
    assert_eq!(def.accent_color, "#0078d4", "Material should feature active accent indicator");
    assert_eq!(def.corner_radius, 4.0);
}

#[test]
fn test_retro_theme_properties() {
    let preset = ThemePreset::Retro;
    let def = ThemeDefinition::from_preset(&preset);
    assert_eq!(def.name, "Retro");
    assert_eq!(def.dwm_backdrop, DwmBackdrop::None);
    assert_eq!(def.opacity, 1.0);
    assert_eq!(def.bg_color, "#c0c0c0", "Retro should use classic silver-gray palette");
    assert!(def.is_retro_beveled, "Retro should feature 3D beveled borders");
    assert!(def.border_width >= 2.0);
    assert_eq!(def.text_color, "#000000", "Retro should feature classic black text");
    assert_eq!(def.corner_radius, 0.0);
}

#[test]
fn test_oled_theme_properties() {
    let preset = ThemePreset::Oled;
    let def = ThemeDefinition::from_preset(&preset);
    assert!(def.name == "OLED" || def.name == "Oled");
    assert_eq!(def.dwm_backdrop, DwmBackdrop::None);
    assert_eq!(def.opacity, 1.0);
    assert_eq!(def.bg_color, "#000000", "OLED should use pure deep black background");
    assert_eq!(def.accent_color, "#ffffff", "OLED should feature high-contrast indicator");
    assert_eq!(def.text_color, "#ffffff");
    assert_eq!(def.corner_radius, 0.0);
}

#[test]
fn test_custom_theme_overrides() {
    let preset = ThemePreset::Custom {
        bg_color: "#123456".to_string(),
        opacity: 0.75,
        border_color: "#654321".to_string(),
    };
    let def = ThemeDefinition::from_preset(&preset);
    assert_eq!(def.name, "Custom");
    assert_eq!(def.bg_color, "#123456");
    assert_eq!(def.opacity, 0.75);
    assert_eq!(def.border_color, "#654321");
    // Acrylic is used when translucent custom theme is configured
    assert_eq!(def.dwm_backdrop, DwmBackdrop::Acrylic);
}

#[test]
fn test_custom_theme_toml_roundtrip() {
    #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
    struct Wrapper {
        theme: ThemePreset,
    }

    let custom = Wrapper {
        theme: ThemePreset::Custom {
            bg_color: "#112233".to_string(),
            opacity: 0.85,
            border_color: "#445566".to_string(),
        },
    };

    let serialized = toml::to_string(&custom).expect("Failed to serialize Custom theme");
    let deserialized: Wrapper = toml::from_str(&serialized).expect("Failed to deserialize Custom theme");
    assert_eq!(custom, deserialized);

    // Also verify deserializing string presets
    let glossy_toml = "theme = \"Glossy\"\n";
    let glossy_wrapper: Wrapper = toml::from_str(glossy_toml).expect("Failed to deserialize Glossy");
    assert_eq!(glossy_wrapper.theme, ThemePreset::Glossy);

    let retro_toml = "theme = \"Retro\"\n";
    let retro_wrapper: Wrapper = toml::from_str(retro_toml).expect("Failed to deserialize Retro");
    assert_eq!(retro_wrapper.theme, ThemePreset::Retro);
}

#[test]
fn test_dwm_backdrop_type_mappings() {
    assert_eq!(DwmBackdrop::from_preset(&ThemePreset::Glossy), DwmBackdrop::Acrylic);
    assert_eq!(DwmBackdrop::from_preset(&ThemePreset::Material), DwmBackdrop::None);
    assert_eq!(DwmBackdrop::from_preset(&ThemePreset::Retro), DwmBackdrop::None);
    assert_eq!(DwmBackdrop::from_preset(&ThemePreset::Oled), DwmBackdrop::None);

    assert_eq!(DwmBackdrop::None.as_dwm_system_backdrop_type(), 1);
    assert_eq!(DwmBackdrop::Mica.as_dwm_system_backdrop_type(), 2);
    assert_eq!(DwmBackdrop::Acrylic.as_dwm_system_backdrop_type(), 3);
    assert_eq!(DwmBackdrop::MicaAlt.as_dwm_system_backdrop_type(), 4);
}

#[test]
fn test_apply_dwm_backdrop_null_or_invalid_handle() {
    // Calling with null/invalid handle should return an error, never panic
    let result = apply_dwm_backdrop(0, DwmBackdrop::Acrylic);
    assert!(result.is_err());
}

#[test]
fn test_theme_style_export_for_slint() {
    let style = ThemeStyle::from_preset(&ThemePreset::Glossy);
    assert_eq!(style.theme_name, "Glossy");
    assert_eq!(style.opacity, 0.6);
    assert_eq!(style.border_width, 1.0);
    assert_eq!(style.corner_radius, 8.0);
    assert!(!style.is_retro);
}
