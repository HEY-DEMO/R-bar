# 4. Native Status Indicators with Native Tray Floating Toggle

We decided to implement custom native Rust widgets for essential system status indicators (Clock, Battery, and Audio volume slider) while providing a toggle to float the native `Shell_TrayWnd` for third-party notification icons, rather than attempting to reverse-engineer or hook Windows 11's XAML-based system tray. Windows 11 22H2+ completely redesigned the system tray into a private XAML island that frequently breaks third-party hooks upon Windows Updates. Building dedicated status widgets ensures guaranteed reliability and instant response for core daily controls.

