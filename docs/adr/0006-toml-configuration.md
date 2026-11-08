# 6. TOML Configuration File in AppData

We decided to persist user configuration, theme settings, and pinned task items in a human-readable `%APPDATA%\taskbar-clone\config.toml` file rather than writing to the Windows Registry. TOML is self-documenting, easily edited or backed up by users, avoids permission issues, and cleanly maps to Rust `serde` structures with zero external C-runtime overhead.

