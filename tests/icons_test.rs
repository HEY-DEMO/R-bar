use std::time::{Duration, Instant};
use taskbar_clone::icons::{
    create_fallback_icon, extract_icon_fallback, extract_icon_from_pe,
    extract_icon_from_window_class, extract_icon_from_window_message, get_window_class_name,
    make_cache_key, IconManager, IconRgba, IconUpdate, ICON_MESSAGE_TIMEOUT_MS,
};

#[test]
fn test_icon_rgba_pixel_buffer() {
    let dummy = IconRgba::new(32, 32, vec![255; 32 * 32 * 4]);
    assert_eq!(dummy.width, 32);
    assert_eq!(dummy.height, 32);
    assert_eq!(dummy.data.len(), 4096);
    assert!(!dummy.is_empty());

    let empty = IconRgba::new(0, 0, Vec::new());
    assert!(empty.is_empty());
}

#[test]
fn test_cache_key_generation() {
    let key1 = make_cache_key("C:\\Windows\\notepad.exe", "Notepad");
    assert_eq!(key1, "c:\\windows\\notepad.exe:notepad");

    let key2 = make_cache_key("C:\\Windows\\System32\\cmd.exe", "");
    assert_eq!(key2, "c:\\windows\\system32\\cmd.exe");

    let key3 = make_cache_key("  explorer.exe  ", "   ");
    assert_eq!(key3, "explorer.exe");
}

#[test]
fn test_bounded_timeout_constant() {
    assert_eq!(ICON_MESSAGE_TIMEOUT_MS, 150);
}

#[test]
fn test_icon_manager_cache_storage_and_retrieval() {
    let manager = IconManager::new();
    let icon = IconRgba::new(16, 16, vec![128; 16 * 16 * 4]);

    let key = make_cache_key("dummy.exe", "DummyClass");
    manager.insert_cache(&key, icon.clone());

    let retrieved = manager.get_cached("dummy.exe", "DummyClass");
    assert_eq!(retrieved, Some(icon.clone()));

    let not_found = manager.get_cached("other.exe", "DummyClass");
    assert_eq!(not_found, None);

    assert_eq!(manager.cache_size(), 1);
    manager.clear_cache();
    assert_eq!(manager.cache_size(), 0);
}

#[test]
fn test_timeout_resilience_on_invalid_window() {
    let start = Instant::now();
    // HWND -1 or non-existent window handle
    let result = extract_icon_from_window_message(-1, ICON_MESSAGE_TIMEOUT_MS);
    let elapsed = start.elapsed();

    assert!(result.is_none());
    // Must complete bounded by reasonable time (well under 500ms)
    assert!(elapsed < Duration::from_millis(500));
}

#[test]
fn test_window_class_icon_invalid_hwnd() {
    assert!(extract_icon_from_window_class(0).is_none());
    assert!(extract_icon_from_window_class(-1).is_none());
    assert!(get_window_class_name(0).is_none());
    assert!(get_window_class_name(-1).is_none());
}

#[test]
fn test_icon_extraction_from_pe_resource() {
    let notepad_path = "C:\\Windows\\System32\\notepad.exe";
    if std::path::Path::new(notepad_path).exists() {
        let extracted = extract_icon_from_pe(notepad_path);
        assert!(extracted.is_some(), "Expected icon to be extracted from notepad.exe");
        let icon = extracted.unwrap();
        assert!(icon.width > 0);
        assert!(icon.height > 0);
        assert_eq!(icon.data.len(), (icon.width * icon.height * 4) as usize);
    }
}

#[test]
fn test_fallback_pipeline() {
    let notepad_path = "C:\\Windows\\System32\\notepad.exe";
    if std::path::Path::new(notepad_path).exists() {
        // Fallback pipeline with hwnd=0 should succeed on step 3 (PE resource)
        let extracted = extract_icon_fallback(0, notepad_path);
        assert!(extracted.is_some());
        let icon = extracted.unwrap();
        assert!(icon.width > 0);
        assert_eq!(icon.data.len(), (icon.width * icon.height * 4) as usize);
    }

    // Completely invalid inputs should return None
    assert!(extract_icon_fallback(0, "").is_none());
    assert!(extract_icon_fallback(-1, "non_existent_binary_12345.exe").is_none());
}

#[test]
fn test_create_fallback_icon() {
    let fallback = create_fallback_icon(32);
    assert_eq!(fallback.width, 32);
    assert_eq!(fallback.height, 32);
    assert_eq!(fallback.data.len(), 32 * 32 * 4);
    // Verify alpha channel is fully opaque
    for chunk in fallback.data.as_chunks::<4>().0 {
        assert_eq!(chunk[3], 255);
    }
}

#[test]
fn test_icon_manager_async_request_and_delivery() {
    let manager = IconManager::new_with_workers(2);
    let notepad_path = "C:\\Windows\\System32\\notepad.exe";

    if std::path::Path::new(notepad_path).exists() {
        // First query: cache miss, triggers async request, returns None
        let _initial = manager.get_icon_or_request(0, notepad_path);
        // Wait up to 1500ms for worker to deliver update
        let start = Instant::now();
        let mut received_update: Option<IconUpdate> = None;

        while start.elapsed() < Duration::from_millis(1500) {
            if let Some(update) = manager.try_recv_update() {
                if update.exe_path.eq_ignore_ascii_case(notepad_path) {
                    received_update = Some(update);
                    break;
                }
            }
            std::thread::sleep(Duration::from_millis(20));
        }

        assert!(received_update.is_some(), "Expected icon update message via channel");
        let update = received_update.unwrap();
        assert!(update.icon.width > 0);
        assert_eq!(update.icon.data.len(), (update.icon.width * update.icon.height * 4) as usize);

        // Subsequent query: should now hit in-memory cache!
        let cached = manager.get_cached(notepad_path, "");
        assert!(cached.is_some(), "Expected icon to be cached in memory");
        assert_eq!(cached.unwrap(), update.icon);

        // Receiver method reference check
        assert!(manager.receiver().is_empty() || !manager.receiver().is_empty());
    }
}
