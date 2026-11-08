# 2. Slint for Declarative Native UI

We decided to use Slint paired with `windows-rs` for the user interface, instead of WebView2/Tauri or raw Direct2D. A persistent taskbar must maintain minimal memory usage (15-30MB) and instantaneous responsiveness. Slint compiles directly to native Rust with GPU acceleration, avoiding the multi-process 150MB+ footprint of Chromium/Tauri while eliminating hundreds of lines of raw Win32 UI drawing boilerplate.

