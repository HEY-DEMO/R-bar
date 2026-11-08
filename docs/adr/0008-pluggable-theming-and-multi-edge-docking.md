# 8. Pluggable Theming Engine and Multi-Edge Geometry

We decided to support four built-in theme presets (Glossy Acrylic, Material Flat, Retro Classic, and OLED Minimalist) combined with 4-edge docking (`Bottom`, `Top`, `Left`, `Right`) configurable via `config.toml` and runtime settings. Slint's reactive property bindings drive layout flow (horizontal for Top/Bottom, vertical for Left/Right), while native Win32 `DwmSetWindowAttribute` enables hardware-rendered Acrylic and Mica backdrops beneath the Slint surface.

