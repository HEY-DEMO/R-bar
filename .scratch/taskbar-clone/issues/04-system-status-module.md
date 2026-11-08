# 04: System Status Module (Clock, Battery & Audio Volume)

**What to build:** Hardware and system status indicators positioned in the tray area of the bar, providing a real-time digital clock/date, live laptop battery percentage and charging state, and an interactive master audio volume slider with mute toggle.

**Blocked by:** 01: Minimal Taskbar Loop & Crash Watchdog

**Status:** ready-for-agent

- [ ] Displays current local time and date updated on a periodic timer.
- [ ] Detects battery presence, remaining charge percentage, and AC power status on laptops via system power APIs.
- [ ] Displays an interactive master audio volume widget reflecting current system volume level.
- [ ] Changing the volume slider or clicking mute updates the master Windows CoreAudio endpoint in real time.
- [ ] Listens for external audio volume changes (e.g. hardware keyboard volume keys) and updates the UI slider accordingly.

