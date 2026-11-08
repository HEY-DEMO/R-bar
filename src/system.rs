use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use windows::core::w;
use windows::Win32::Foundation::{BOOL, HWND};
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{
    eMultimedia, eRender, IMMDevice, IMMDeviceEnumerator, MMDeviceEnumerator,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_APARTMENTTHREADED,
};
use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
use windows::Win32::System::SystemInformation::GetLocalTime;
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowExW, FindWindowW, IsWindowVisible, SetWindowPos, ShowWindow, HWND_TOPMOST, SWP_NOMOVE,
    SWP_NOSIZE, SW_HIDE, SW_SHOW,
};

/// Represents snapshot of power and battery status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BatteryStatus {
    /// Battery charge percentage (0-100), or None if no battery is detected.
    pub percent: Option<u8>,
    /// Whether the battery is actively charging or connected to AC line power.
    pub is_charging: bool,
    /// Whether the system is plugged into AC power.
    pub ac_line_online: bool,
    /// Whether a physical battery is present in the system.
    pub has_battery: bool,
}

/// Represents current aggregated system status indicators.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SystemStatus {
    /// Formatted local time string (e.g. "14:05").
    pub time_str: String,
    /// Formatted local date string (e.g. "2026-11-08").
    pub date_str: String,
    /// Battery remaining percentage (0-100), or None if desktop/no battery.
    pub battery_percent: Option<u8>,
    /// Whether system is connected to AC power / charging.
    pub is_charging: bool,
    /// Master volume level percentage (0-100).
    pub volume_percent: u8,
    /// Whether the master audio output is currently muted.
    pub is_muted: bool,
}

/// Formats the current local time and date.
/// Returns `(time_str, date_str)` in local system time.
pub fn format_clock() -> (String, String) {
    let st = unsafe { GetLocalTime() };
    let time_str = format!("{:02}:{:02}", st.wHour, st.wMinute);
    let date_str = format!("{:04}-{:02}-{:02}", st.wYear, st.wMonth, st.wDay);
    (time_str, date_str)
}

/// Queries system power and battery status via Win32 `GetSystemPowerStatus`.
pub fn get_battery_status() -> BatteryStatus {
    let mut sps = SYSTEM_POWER_STATUS::default();
    let ok = unsafe { GetSystemPowerStatus(&mut sps) };

    if ok.is_err() {
        return BatteryStatus {
            percent: None,
            is_charging: false,
            ac_line_online: false,
            has_battery: false,
        };
    }

    let ac_line_online = sps.ACLineStatus == 1;
    // BatteryFlag: bit 7 (128) = no system battery; bit 3 (8) = charging; 255 = unknown
    let no_battery = (sps.BatteryFlag & 128) != 0 || sps.BatteryLifePercent == 255;
    let is_charging = (sps.BatteryFlag & 8) != 0 || ac_line_online;

    if no_battery {
        BatteryStatus {
            percent: None,
            is_charging,
            ac_line_online,
            has_battery: false,
        }
    } else {
        let percent = sps.BatteryLifePercent.min(100);
        BatteryStatus {
            percent: Some(percent),
            is_charging,
            ac_line_online,
            has_battery: true,
        }
    }
}

/// Clamps volume level strictly within `[0.0, 1.0]`.
#[inline]
pub fn clamp_volume(level: f32) -> f32 {
    if level.is_nan() {
        0.0
    } else {
        level.clamp(0.0, 1.0)
    }
}

/// COM error code indicating that the concurrency model cannot be changed once set.
pub const RPC_E_CHANGED_MODE: i32 = 0x80010106_u32 as i32;

/// Helper to execute a closure with the default master audio endpoint volume interface.
/// Safely manages thread COM initialization.
fn with_master_audio_endpoint<T, F>(f: F) -> Result<T, isize>
where
    F: FnOnce(&IAudioEndpointVolume) -> windows::core::Result<T>,
{
    unsafe {
        // Initialize COM for this thread with apartment threading (STA) to remain
        // compatible with winit / Slint UI and OLE drag-and-drop.
        let hr = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        if hr.is_err() && hr.0 != RPC_E_CHANGED_MODE {
            // Continue anyway to attempt using existing COM context
        }

        let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
            .map_err(|e| e.code().0 as isize)?;

        let device: IMMDevice = enumerator
            .GetDefaultAudioEndpoint(eRender, eMultimedia)
            .map_err(|e| e.code().0 as isize)?;

        let endpoint_volume: IAudioEndpointVolume = device
            .Activate(CLSCTX_ALL, None)
            .map_err(|e| e.code().0 as isize)?;

        f(&endpoint_volume).map_err(|e| e.code().0 as isize)
    }
}

/// Retrieves the current master audio volume as a scalar between `0.0` and `1.0`.
pub fn get_master_volume() -> Result<f32, isize> {
    with_master_audio_endpoint(|endpoint| {
        let level = unsafe { endpoint.GetMasterVolumeLevelScalar()? };
        Ok(clamp_volume(level))
    })
}

/// Sets the master audio volume level scalar between `0.0` and `1.0`.
/// Input is clamped to valid range.
pub fn set_master_volume(level: f32) -> Result<(), isize> {
    let clamped = clamp_volume(level);
    with_master_audio_endpoint(|endpoint| {
        unsafe {
            endpoint.SetMasterVolumeLevelScalar(clamped, std::ptr::null())?;
        }
        Ok(())
    })
}

/// Retrieves the master audio mute state.
pub fn get_master_mute() -> Result<bool, isize> {
    with_master_audio_endpoint(|endpoint| {
        let mute = unsafe { endpoint.GetMute()? };
        Ok(mute.as_bool())
    })
}

/// Sets the master audio mute state.
pub fn set_master_mute(mute: bool) -> Result<(), isize> {
    with_master_audio_endpoint(|endpoint| {
        unsafe {
            endpoint.SetMute(BOOL::from(mute), std::ptr::null())?;
        }
        Ok(())
    })
}

/// Toggles the master audio mute state and returns the new mute state.
pub fn toggle_master_mute() -> Result<bool, isize> {
    let current = get_master_mute()?;
    let new_mute = !current;
    set_master_mute(new_mute)?;
    Ok(new_mute)
}

/// Handle to an active background volume listener thread.
pub struct VolumeListenerHandle {
    stop_signal: Arc<AtomicBool>,
    thread_handle: Option<JoinHandle<()>>,
}

impl VolumeListenerHandle {
    /// Stops the listener thread and waits for it to terminate.
    pub fn stop(mut self) {
        self.stop_signal.store(true, Ordering::SeqCst);
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for VolumeListenerHandle {
    fn drop(&mut self) {
        self.stop_signal.store(true, Ordering::SeqCst);
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }
}

/// Starts a background listener polling for external audio volume and mute changes.
/// Calls `callback(volume_scalar, is_muted)` whenever external changes are detected.
pub fn start_volume_listener<F>(poll_interval: Duration, callback: F) -> VolumeListenerHandle
where
    F: Fn(f32, bool) + Send + Sync + 'static,
{
    let stop_signal = Arc::new(AtomicBool::new(false));
    let stop_clone = Arc::clone(&stop_signal);

    let thread_handle = thread::spawn(move || {
        let mut last_vol = get_master_volume().unwrap_or(0.5);
        let mut last_mute = get_master_mute().unwrap_or(false);

        while !stop_clone.load(Ordering::SeqCst) {
            thread::sleep(poll_interval);

            if let Ok(current_vol) = get_master_volume() {
                let current_mute = get_master_mute().unwrap_or(last_mute);

                // Check for significant volume change (> 0.5%) or mute change
                let vol_changed = (current_vol - last_vol).abs() > 0.005;
                let mute_changed = current_mute != last_mute;

                if vol_changed || mute_changed {
                    last_vol = current_vol;
                    last_mute = current_mute;
                    callback(current_vol, current_mute);
                }
            }
        }
    });

    VolumeListenerHandle {
        stop_signal,
        thread_handle: Some(thread_handle),
    }
}

/// Aggregates all current system indicators into a single snapshot.
/// Provides safe fallbacks when audio or battery devices are unavailable.
pub fn poll_system_status() -> SystemStatus {
    let (time_str, date_str) = format_clock();
    let battery = get_battery_status();

    let (vol, muted) = match get_master_volume() {
        Ok(v) => {
            let m = get_master_mute().unwrap_or(false);
            (v, m)
        }
        Err(_) => (0.5, false), // Safe fallback
    };

    let volume_percent = (vol * 100.0).round() as u8;

    SystemStatus {
        time_str,
        date_str,
        battery_percent: battery.percent,
        is_charging: battery.is_charging,
        volume_percent: volume_percent.min(100),
        is_muted: muted,
    }
}

/// Finds the native Windows taskbar or tray window handle.
pub fn find_native_tray_window() -> Option<HWND> {
    unsafe {
        if let Ok(hwnd) = FindWindowW(w!("Shell_TrayWnd"), None) {
            if !hwnd.0.is_null() {
                return Some(hwnd);
            }
        }
    }
    None
}

/// Finds the native Windows notification tray window (`TrayNotifyWnd`) inside the shell tray.
pub fn find_tray_notify_window() -> Option<HWND> {
    unsafe {
        let tray_wnd = find_native_tray_window()?;
        if let Ok(hwnd) = FindWindowExW(tray_wnd, HWND(std::ptr::null_mut()), w!("TrayNotifyWnd"), None) {
            if !hwnd.0.is_null() {
                return Some(hwnd);
            }
        }
    }
    None
}

/// Returns whether the native taskbar notification area is currently visible.
pub fn is_native_tray_visible() -> bool {
    if let Some(hwnd) = find_native_tray_window() {
        unsafe { IsWindowVisible(hwnd).as_bool() }
    } else {
        false
    }
}

/// Toggles visibility of the native Windows taskbar / notification area (`TrayNotifyWnd` inside `Shell_TrayWnd`).
///
/// When unhiding, positions the native tray topmost so background notification icons float
/// above normal application windows. Returns the new visibility state.
pub fn toggle_native_tray() -> bool {
    if let Some(hwnd) = find_native_tray_window() {
        let currently_visible = unsafe { IsWindowVisible(hwnd).as_bool() };
        unsafe {
            if currently_visible {
                let _ = ShowWindow(hwnd, SW_HIDE);
                if let Some(notify_hwnd) = find_tray_notify_window() {
                    let _ = ShowWindow(notify_hwnd, SW_HIDE);
                }
            } else {
                let _ = ShowWindow(hwnd, SW_SHOW);
                let _ = SetWindowPos(
                    hwnd,
                    HWND_TOPMOST,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE,
                );
                if let Some(notify_hwnd) = find_tray_notify_window() {
                    let _ = ShowWindow(notify_hwnd, SW_SHOW);
                }
            }
        }
        !currently_visible
    } else {
        false
    }
}
