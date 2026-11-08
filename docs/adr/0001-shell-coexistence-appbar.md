# 1. Shell Co-existence via AppBar and Window Hiding

We decided to run alongside `explorer.exe` by hiding `Shell_TrayWnd` and registering our window as a Win32 AppBar, rather than replacing `explorer.exe` as the Windows logon shell. Fully replacing Explorer breaks vital Windows OS services (Start Menu host, Quick Settings, file dialogs, and shell shortcuts), whereas hiding the native window achieves complete visual replacement with minimal risk.

