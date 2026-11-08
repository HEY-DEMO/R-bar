# 09: Pluggable Theming Engine (Glossy, Material, Retro, OLED)

**What to build:** Visual personalization options supporting four built-in theme presets (`Glossy`, `Material`, `Retro`, `OLED`) selectable via configuration, integrating directly with Windows 11 Desktop Window Manager backdrop materials.

**Blocked by:** 02: TOML Configuration & 4-Edge Geometry Engine

**Status:** ready-for-agent

- [ ] Applies the `Glossy` theme preset by enabling Windows 11 DWM backdrop blur (Acrylic/Mica) beneath a translucent surface with subtle highlight borders.
- [ ] Applies the `Material` theme preset featuring clean, solid surfaces, subtle elevation drop shadows, and active accent indicators.
- [ ] Applies the `Retro` theme preset featuring classic silver-gray palettes and 3D beveled borders.
- [ ] Applies the `OLED` theme preset with pure deep black background surfaces and high-contrast indicators.
- [ ] Supports custom background color codes and surface opacity overrides defined in the configuration file.

