# 09: Pluggable Theming Engine (Glossy, Material, Retro, OLED)

**What to build:** Visual personalization options supporting four built-in theme presets (`Glossy`, `Material`, `Retro`, `OLED`) selectable via configuration, integrating directly with Windows 11 Desktop Window Manager backdrop materials.

**Blocked by:** 02: TOML Configuration & 4-Edge Geometry Engine

**Status:** closed

- [x] Applies the `Glossy` theme preset by enabling Windows 11 DWM backdrop blur (Acrylic/Mica) beneath a translucent surface with subtle highlight borders.
- [x] Applies the `Material` theme preset featuring clean, solid surfaces, subtle elevation drop shadows, and active accent indicators.
- [x] Applies the `Retro` theme preset featuring classic silver-gray palettes and 3D beveled borders.
- [x] Applies the `OLED` theme preset with pure deep black background surfaces and high-contrast indicators.
- [x] Supports custom background color codes and surface opacity overrides defined in the configuration file.

## Comments

### Resolution
- Merged branch `ticket-09` into `feat/taskbar-clone` at commit `00ab534` (source commit `e46b511`).
- Implemented `src/theme.rs`:
  - `ThemePreset` enum supporting `Glossy`, `Material`, `Retro`, `Oled`, and `Custom { bg_color, opacity, border_color }` variants with serde round-trip serialization/deserialization.
  - `DwmBackdrop` mapping theme presets to Windows 11 Desktop Window Manager system backdrop materials (Acrylic `DWMSBT_TRANSIENTWINDOW`, Mica `DWMSBT_MAINWINDOW`, MicaAlt `DWMSBT_TABBEDWINDOW`, None `DWMSBT_NONE`).
  - `apply_dwm_backdrop()` integrating with Win32 `DwmSetWindowAttribute(DWMWA_SYSTEMBACKDROP_TYPE)` with safe error handling on invalid/null handles.
  - `ThemeDefinition` and `ThemeStyle` structures resolving complete theme palettes and styling properties for runtime UI consumption.
- Implemented Slint theme palette system:
  - `ui/theme.slint` with `ThemePalette` struct and `Theme` global providing predefined palettes for `Glossy`, `Material`, `Retro`, and `Oled`.
  - Integrated theme palettes with `ui/taskbar.slint`, preserving start menu triggers and pinned launcher items from ticket 03.
- Extended configuration system in `src/config.rs`:
  - Exposed `ThemePreset` from `crate::theme` and wired into `Config.theme`.
  - Added unit and integration tests in `tests/theme_test.rs` covering all presets, custom overrides, TOML roundtrips, DWM backdrop mappings, and Slint style exports.
- Verified all 47 tests pass via `cargo test`.
