# 02: TOML Configuration & 4-Edge Geometry Engine

**What to build:** A human-editable configuration system that reads and saves user settings from the application data directory, computes taskbar coordinates for any screen edge (`Bottom`, `Top`, `Left`, `Right`), and dynamically switches between horizontal row and vertical column layouts.

**Blocked by:** 01: Minimal Taskbar Loop & Crash Watchdog

**Status:** resolved

- [x] Generates a default `config.toml` file in the user's application data directory if none exists.
- [x] Parses configuration options for dock edge (`Bottom`, `Top`, `Left`, `Right`), task alignment (`Left`, `Center`), bar thickness, and theme preset.
- [x] Computes exact window coordinate rectangles for all four screen edges based on monitor bounds and configured thickness.
- [x] Adjusts the UI layout orientation automatically: horizontal row for Top/Bottom edges, vertical column for Left/Right edges.
- [x] Fallbacks gracefully to sensible defaults when configuration files are missing or contain malformed syntax.
