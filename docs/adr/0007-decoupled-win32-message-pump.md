# 7. Dedicated Win32 Message Pump with Slint Event Loop Dispatch

We decided to run an invisible Win32 message-only window on a dedicated OS thread for receiving shell hook notifications (`WM_SHELLHOOK`), display changes, and power events, dispatching updates to Slint via `slint::invoke_from_event_loop`. This decouples bursty Win32 OS messages and slow COM event callbacks from Slint's rendering thread, ensuring smooth 60/120 FPS UI animations with zero frame hitching.

