use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use taskbar_clone::system::{
    clamp_volume, find_native_tray_window, format_clock, get_battery_status, get_master_mute,
    get_master_volume, is_native_tray_visible, poll_system_status, set_master_mute,
    set_master_volume, start_volume_listener, toggle_master_mute, toggle_native_tray,
    BatteryStatus, SystemStatus,
};

#[test]
fn test_clock_formatting() {
    let (time_str, date_str) = format_clock();
    assert!(!time_str.is_empty(), "Time string should not be empty");
    assert!(!date_str.is_empty(), "Date string should not be empty");
    assert!(
        time_str.contains(':'),
        "Time string should contain colon separator: {}",
        time_str
    );
    assert!(
        date_str.contains('-'),
        "Date string should contain hyphen separator: {}",
        date_str
    );
}

#[test]
fn test_system_status_query() {
    let status: SystemStatus = poll_system_status();
    assert!(!status.time_str.is_empty());
    assert!(!status.date_str.is_empty());
    if let Some(percent) = status.battery_percent {
        assert!(percent <= 100, "Battery percentage must be <= 100");
    }
    assert!(
        status.volume_percent <= 100,
        "Volume percent must be clamped to 0..=100"
    );
}

#[test]
fn test_volume_bounds_clamping() {
    assert_eq!(clamp_volume(-0.5), 0.0);
    assert_eq!(clamp_volume(0.0), 0.0);
    assert_eq!(clamp_volume(0.42), 0.42);
    assert_eq!(clamp_volume(1.0), 1.0);
    assert_eq!(clamp_volume(1.5), 1.0);
    assert_eq!(clamp_volume(f32::NAN), 0.0);
}

#[test]
fn test_battery_status_fallback() {
    let battery: BatteryStatus = get_battery_status();
    if battery.has_battery {
        assert!(battery.percent.is_some());
        assert!(battery.percent.unwrap() <= 100);
    } else {
        assert!(battery.percent.is_none());
    }
}

#[test]
fn test_audio_volume_api() {
    // If an audio device is available on this system, test getting and setting volume
    match get_master_volume() {
        Ok(vol) => {
            assert!(
                (0.0..=1.0).contains(&vol),
                "Master volume scalar must be in 0.0..=1.0: {}",
                vol
            );
            // Verify setting valid volume succeeds
            let set_res = set_master_volume(vol);
            assert!(set_res.is_ok(), "Setting current volume should succeed");

            // Verify volume clamping on set
            let clamped_res = set_master_volume(1.2);
            assert!(clamped_res.is_ok(), "Setting volume > 1.0 should clamp and succeed");
            let _ = set_master_volume(vol); // restore original
        }
        Err(err) => {
            // In headless/no-audio environments, error code should be non-zero
            assert_ne!(err, 0, "Error code should be non-zero on failure");
        }
    }

    match get_master_mute() {
        Ok(muted) => {
            // Test toggling mute twice restores original state
            if let Ok(new_mute) = toggle_master_mute() {
                assert_eq!(new_mute, !muted);
                let restored = toggle_master_mute().expect("Restoring mute should succeed");
                assert_eq!(restored, muted);
            }
            let _ = set_master_mute(muted); // safety restore
        }
        Err(err) => {
            assert_ne!(err, 0);
        }
    }
}

#[test]
fn test_volume_listener_lifecycle() {
    let trigger_count = Arc::new(AtomicUsize::new(0));
    let count_clone = Arc::clone(&trigger_count);

    let handle = start_volume_listener(Duration::from_millis(50), move |_vol, _mute| {
        count_clone.fetch_add(1, Ordering::SeqCst);
    });

    // Let it run for a brief moment
    thread::sleep(Duration::from_millis(120));
    handle.stop();
}

#[test]
fn test_native_tray_toggle() {
    let has_tray = find_native_tray_window().is_some();
    if has_tray {
        let initial = is_native_tray_visible();
        let toggled = toggle_native_tray();
        assert_ne!(initial, toggled, "Toggling tray should flip visibility state");
        let reverted = toggle_native_tray();
        assert_eq!(initial, reverted, "Toggling tray twice should restore initial state");
    } else {
        let toggled = toggle_native_tray();
        assert!(!toggled, "Toggle should return false if no tray window is found");
    }
}
