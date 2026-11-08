# 04: System Status Module (Clock, Battery & Audio Volume)

**What to build:** Hardware and system status indicators positioned in the tray area of the bar, providing a real-time digital clock/date, live laptop battery percentage and charging state, and an interactive master audio volume slider with mute toggle.

**Blocked by:** 01: Minimal Taskbar Loop & Crash Watchdog

**Status:** closed

- [x] Displays current local time and date updated on a periodic timer.
- [x] Detects battery presence, remaining charge percentage, and AC power status on laptops via system power APIs.
- [x] Displays an interactive master audio volume widget reflecting current system volume level.
- [x] Changing the volume slider or clicking mute updates the master Windows CoreAudio endpoint in real time.
- [x] Listens for external audio volume changes (e.g. hardware keyboard volume keys) and updates the UI slider accordingly.

## Comments

### Resolution
- Merged branch `ticket-04` into `feat/taskbar-clone` at merge commit `f2bae52` (source commit `347fb79`).
- Implemented system status module in `src/system.rs`:
  - `ClockService` for digital time and date formatting.
  - `PowerStatusService` for Windows power status API (`GetSystemPowerStatus`) querying battery charge percentage and AC line state.
  - `AudioVolumeService` and COM-based Windows CoreAudio endpoint volume control (`IAudioEndpointVolume`, `IAudioEndpointVolumeCallback`) with real-time hardware volume change notification listening and UI slider synchronization.
  - Native Windows tray notification area toggle (`toggle_native_tray_overflow_window` / `toggle_native_quick_settings`).
- Verified with 7 integration tests in `tests/system_test.rs` and the full test suite (`cargo test` - all 21 tests passing).
