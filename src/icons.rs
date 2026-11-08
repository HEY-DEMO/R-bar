use crossbeam_channel::{unbounded, Receiver, Sender};
use std::collections::{HashMap, HashSet};
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::thread::{self, JoinHandle};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits, GetObjectW, BITMAP, BITMAPINFO,
    BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, DIB_USAGE, HBITMAP, HDC, HGDIOBJ, RGBQUAD,
};
use windows::Win32::UI::Shell::ExtractIconExW;
use windows::Win32::UI::WindowsAndMessaging::{
    DestroyIcon, GetClassLongPtrW, GetClassNameW, GetIconInfo, IsWindow, SendMessageTimeoutW,
    GCLP_HICON, GCLP_HICONSM, HICON, ICONINFO, ICON_BIG, ICON_SMALL, ICON_SMALL2,
    SEND_MESSAGE_TIMEOUT_FLAGS, SMTO_ABORTIFHUNG, SMTO_BLOCK, WM_GETICON,
};

/// Strict 150ms timeout for window icon messages to avoid hanging on unresponsive targets.
pub const ICON_MESSAGE_TIMEOUT_MS: u32 = 150;

/// Decoded 32-bit RGBA pixel buffer representing an icon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IconRgba {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

impl IconRgba {
    pub fn new(width: u32, height: u32, data: Vec<u8>) -> Self {
        Self {
            width,
            height,
            data,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0 || self.data.is_empty()
    }
}

/// Message emitted when a background worker successfully resolves an icon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IconUpdate {
    pub hwnd: isize,
    pub exe_path: String,
    pub class_name: String,
    pub key: String,
    pub icon: IconRgba,
}

/// Work item dispatched to background icon workers.
#[derive(Debug, Clone)]
struct IconRequest {
    hwnd: isize,
    exe_path: String,
    class_name: String,
    key: String,
}

/// Constructs a normalized cache key from executable path and window class name.
pub fn make_cache_key(exe_path: &str, class_name: &str) -> String {
    let normalized_path = exe_path.trim().to_lowercase();
    let normalized_class = class_name.trim().to_lowercase();

    if normalized_class.is_empty() {
        normalized_path
    } else {
        format!("{}:{}", normalized_path, normalized_class)
    }
}

/// Retrieves the window class name for a given HWND.
pub fn get_window_class_name(hwnd: isize) -> Option<String> {
    if hwnd == 0 {
        return None;
    }
    let h = HWND(hwnd as *mut _);
    if unsafe { !IsWindow(h).as_bool() } {
        return None;
    }

    let mut buffer = [0u16; 256];
    let len = unsafe { GetClassNameW(h, &mut buffer) };
    if len > 0 {
        Some(String::from_utf16_lossy(&buffer[..len as usize]))
    } else {
        None
    }
}

/// Extracts an application icon via `WM_GETICON` with a strict bounded timeout.
pub fn extract_icon_from_window_message(hwnd: isize, timeout_ms: u32) -> Option<IconRgba> {
    if hwnd <= 0 {
        return None;
    }
    let h = HWND(hwnd as *mut _);
    if unsafe { !IsWindow(h).as_bool() } {
        return None;
    }

    let mut lresult: usize = 0;
    let flags: SEND_MESSAGE_TIMEOUT_FLAGS = SMTO_ABORTIFHUNG | SMTO_BLOCK;

    // Try ICON_BIG first
    let res = unsafe {
        SendMessageTimeoutW(
            h,
            WM_GETICON,
            WPARAM(ICON_BIG as usize),
            LPARAM(0),
            flags,
            timeout_ms,
            Some(&mut lresult),
        )
    };

    let mut hicon = if res != LRESULT(0) && lresult != 0 {
        HICON(lresult as *mut _)
    } else {
        HICON(std::ptr::null_mut())
    };

    // If ICON_BIG returned nothing, try ICON_SMALL2 / ICON_SMALL with bounded timeout
    if hicon.0.is_null() {
        let mut small_result: usize = 0;
        let res_small = unsafe {
            SendMessageTimeoutW(
                h,
                WM_GETICON,
                WPARAM(ICON_SMALL2 as usize),
                LPARAM(0),
                flags,
                timeout_ms.min(50),
                Some(&mut small_result),
            )
        };

        if res_small != LRESULT(0) && small_result != 0 {
            hicon = HICON(small_result as *mut _);
        } else {
            let res_sm = unsafe {
                SendMessageTimeoutW(
                    h,
                    WM_GETICON,
                    WPARAM(ICON_SMALL as usize),
                    LPARAM(0),
                    flags,
                    timeout_ms.min(50),
                    Some(&mut small_result),
                )
            };
            if res_sm != LRESULT(0) && small_result != 0 {
                hicon = HICON(small_result as *mut _);
            }
        }
    }

    if !hicon.0.is_null() {
        hicon_to_rgba(hicon)
    } else {
        None
    }
}

/// Extracts an icon handle from the window class attributes (`GCLP_HICON` / `GCLP_HICONSM`).
pub fn extract_icon_from_window_class(hwnd: isize) -> Option<IconRgba> {
    if hwnd <= 0 {
        return None;
    }
    let h = HWND(hwnd as *mut _);
    if unsafe { !IsWindow(h).as_bool() } {
        return None;
    }

    let val = unsafe { GetClassLongPtrW(h, GCLP_HICON) };
    let mut hicon = if val != 0 {
        HICON(val as *mut _)
    } else {
        HICON(std::ptr::null_mut())
    };

    if hicon.0.is_null() {
        let sm_val = unsafe { GetClassLongPtrW(h, GCLP_HICONSM) };
        if sm_val != 0 {
            hicon = HICON(sm_val as *mut _);
        }
    }

    if !hicon.0.is_null() {
        hicon_to_rgba(hicon)
    } else {
        None
    }
}

/// Extracts an icon directly from an executable file's PE resource headers.
pub fn extract_icon_from_pe(exe_path: &str) -> Option<IconRgba> {
    if exe_path.trim().is_empty() {
        return None;
    }

    let path_wide: Vec<u16> = exe_path.encode_utf16().chain(std::iter::once(0)).collect();
    let mut large_icon = HICON(std::ptr::null_mut());
    let mut small_icon = HICON(std::ptr::null_mut());

    let count = unsafe {
        ExtractIconExW(
            PCWSTR(path_wide.as_ptr()),
            0,
            Some(&mut large_icon),
            Some(&mut small_icon),
            1,
        )
    };

    if count == 0 {
        return None;
    }

    let result = if !large_icon.0.is_null() {
        hicon_to_rgba(large_icon)
    } else if !small_icon.0.is_null() {
        hicon_to_rgba(small_icon)
    } else {
        None
    };

    // Clean up handles created by ExtractIconExW
    if !large_icon.0.is_null() {
        let _ = unsafe { DestroyIcon(large_icon) };
    }
    if !small_icon.0.is_null() {
        let _ = unsafe { DestroyIcon(small_icon) };
    }

    result
}

/// Complete fallback icon extraction pipeline:
/// 1. Window messages with timeout (`WM_GETICON` with 150ms timeout)
/// 2. Window class icon handles (`GetClassLongPtrW(GCLP_HICON)`)
/// 3. Executable PE resources (`ExtractIconExW`)
pub fn extract_icon_fallback(hwnd: isize, exe_path: &str) -> Option<IconRgba> {
    // Step 1: WM_GETICON with timeout
    if hwnd > 0 {
        if let Some(icon) = extract_icon_from_window_message(hwnd, ICON_MESSAGE_TIMEOUT_MS) {
            return Some(icon);
        }
    }

    // Step 2: Window class icon handle
    if hwnd > 0 {
        if let Some(icon) = extract_icon_from_window_class(hwnd) {
            return Some(icon);
        }
    }

    // Step 3: Executable PE resource
    if !exe_path.is_empty() {
        if let Some(icon) = extract_icon_from_pe(exe_path) {
            return Some(icon);
        }
    }

    None
}

/// Converts a Win32 `HICON` into a 32-bit RGBA pixel buffer.
pub fn hicon_to_rgba(hicon: HICON) -> Option<IconRgba> {
    if hicon.0.is_null() {
        return None;
    }

    let mut icon_info = ICONINFO::default();
    if unsafe { GetIconInfo(hicon, &mut icon_info).is_err() } {
        return None;
    }

    // RAII guard for GDI bitmap cleanup
    struct GdiBitmapGuard(HBITMAP);
    impl Drop for GdiBitmapGuard {
        fn drop(&mut self) {
            if !self.0 .0.is_null() {
                let _ = unsafe { DeleteObject(HGDIOBJ(self.0 .0)) };
            }
        }
    }

    let _mask_guard = GdiBitmapGuard(icon_info.hbmMask);
    let _color_guard = GdiBitmapGuard(icon_info.hbmColor);

    let hdc = unsafe { CreateCompatibleDC(None) };
    if hdc.0.is_null() {
        return None;
    }

    struct DcGuard(HDC);
    impl Drop for DcGuard {
        fn drop(&mut self) {
            if !self.0 .0.is_null() {
                let _ = unsafe { DeleteDC(self.0) };
            }
        }
    }
    let _dc_guard = DcGuard(hdc);

    // Determine dimensions from bitmaps
    let (width, height, is_color) = if !icon_info.hbmColor.0.is_null() {
        let mut bmp = BITMAP::default();
        let bytes = unsafe {
            GetObjectW(
                HGDIOBJ(icon_info.hbmColor.0),
                std::mem::size_of::<BITMAP>() as i32,
                Some(&mut bmp as *mut _ as *mut c_void),
            )
        };
        if bytes == 0 || bmp.bmWidth <= 0 || bmp.bmHeight <= 0 {
            return None;
        }
        (bmp.bmWidth as u32, bmp.bmHeight as u32, true)
    } else if !icon_info.hbmMask.0.is_null() {
        let mut bmp = BITMAP::default();
        let bytes = unsafe {
            GetObjectW(
                HGDIOBJ(icon_info.hbmMask.0),
                std::mem::size_of::<BITMAP>() as i32,
                Some(&mut bmp as *mut _ as *mut c_void),
            )
        };
        if bytes == 0 || bmp.bmWidth <= 0 || bmp.bmHeight <= 0 {
            return None;
        }
        // In monochrome icons, mask height includes both AND and XOR masks
        (bmp.bmWidth as u32, (bmp.bmHeight / 2) as u32, false)
    } else {
        return None;
    };

    if width == 0 || height == 0 {
        return None;
    }

    let pixel_count = (width * height) as usize;
    let mut rgba_buffer = vec![0u8; pixel_count * 4];

    let mut bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width as i32,
            biHeight: -(height as i32), // Top-down scan lines
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        },
        bmiColors: [RGBQUAD::default(); 1],
    };

    if is_color {
        let lines = unsafe {
            GetDIBits(
                hdc,
                icon_info.hbmColor,
                0,
                height,
                Some(rgba_buffer.as_mut_ptr() as *mut c_void),
                &mut bmi,
                DIB_USAGE(DIB_RGB_COLORS.0),
            )
        };
        if lines == 0 {
            return None;
        }

        // Check whether color bitmap already includes valid alpha values
        let has_alpha = rgba_buffer.as_chunks::<4>().0.iter().any(|chunk| chunk[3] > 0);

        if has_alpha {
            // Swap BGRA to RGBA
            for chunk in rgba_buffer.as_chunks_mut::<4>().0 {
                chunk.swap(0, 2);
            }
        } else {
            // Extract AND mask to determine transparency
            let mut mask_buffer = vec![0u8; pixel_count * 4];
            let mut mask_bmi = bmi;
            let mask_lines = unsafe {
                GetDIBits(
                    hdc,
                    icon_info.hbmMask,
                    0,
                    height,
                    Some(mask_buffer.as_mut_ptr() as *mut c_void),
                    &mut mask_bmi,
                    DIB_USAGE(DIB_RGB_COLORS.0),
                )
            };

            for i in 0..pixel_count {
                let offset = i * 4;
                let b = rgba_buffer[offset];
                let g = rgba_buffer[offset + 1];
                let r = rgba_buffer[offset + 2];
                rgba_buffer[offset] = r;
                rgba_buffer[offset + 1] = g;
                rgba_buffer[offset + 2] = b;

                if mask_lines != 0 {
                    // AND mask: 1 is transparent, 0 is opaque
                    let is_transparent = (mask_buffer[offset]
                        | mask_buffer[offset + 1]
                        | mask_buffer[offset + 2])
                        != 0;
                    rgba_buffer[offset + 3] = if is_transparent { 0 } else { 255 };
                } else {
                    rgba_buffer[offset + 3] = 255;
                }
            }
        }
    } else {
        // Monochrome icon: hbmMask has height * 2 lines (AND mask followed by XOR mask)
        let mut full_mask_buffer = vec![0u8; pixel_count * 8];
        let mut mono_bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width as i32,
                biHeight: -((height * 2) as i32),
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [RGBQUAD::default(); 1],
        };

        let lines = unsafe {
            GetDIBits(
                hdc,
                icon_info.hbmMask,
                0,
                height * 2,
                Some(full_mask_buffer.as_mut_ptr() as *mut c_void),
                &mut mono_bmi,
                DIB_USAGE(DIB_RGB_COLORS.0),
            )
        };
        if lines == 0 {
            return None;
        }

        for i in 0..pixel_count {
            let and_offset = i * 4;
            let xor_offset = (pixel_count + i) * 4;
            let and_pixel = full_mask_buffer[and_offset]
                | full_mask_buffer[and_offset + 1]
                | full_mask_buffer[and_offset + 2];
            let xor_pixel = full_mask_buffer[xor_offset]
                | full_mask_buffer[xor_offset + 1]
                | full_mask_buffer[xor_offset + 2];

            let dest_offset = i * 4;
            if and_pixel != 0 {
                // Transparent
                rgba_buffer[dest_offset] = 0;
                rgba_buffer[dest_offset + 1] = 0;
                rgba_buffer[dest_offset + 2] = 0;
                rgba_buffer[dest_offset + 3] = 0;
            } else {
                // Opaque
                let val = if xor_pixel != 0 { 255 } else { 0 };
                rgba_buffer[dest_offset] = val;
                rgba_buffer[dest_offset + 1] = val;
                rgba_buffer[dest_offset + 2] = val;
                rgba_buffer[dest_offset + 3] = 255;
            }
        }
    }

    Some(IconRgba {
        width,
        height,
        data: rgba_buffer,
    })
}

/// Fallback placeholder icon generator.
pub fn create_fallback_icon(size: u32) -> IconRgba {
    let size = if size == 0 { 32 } else { size };
    let mut data = vec![0u8; (size * size * 4) as usize];
    for y in 0..size {
        for x in 0..size {
            let offset = ((y * size + x) * 4) as usize;
            let is_border = x == 0 || y == 0 || x == size - 1 || y == size - 1;
            if is_border {
                data[offset] = 120;
                data[offset + 1] = 120;
                data[offset + 2] = 120;
                data[offset + 3] = 255;
            } else {
                data[offset] = 220;
                data[offset + 1] = 220;
                data[offset + 2] = 220;
                data[offset + 3] = 255;
            }
        }
    }
    IconRgba {
        width: size,
        height: size,
        data,
    }
}

/// Asynchronous off-thread icon management and caching engine.
pub struct IconManager {
    cache: Arc<RwLock<HashMap<String, IconRgba>>>,
    in_flight: Arc<Mutex<HashSet<String>>>,
    request_sender: Sender<IconRequest>,
    update_receiver: Receiver<IconUpdate>,
    running: Arc<AtomicBool>,
    _worker_threads: Vec<JoinHandle<()>>,
}

impl IconManager {
    /// Creates a new `IconManager` with the default background worker pool size (2 workers).
    pub fn new() -> Self {
        Self::new_with_workers(2)
    }

    /// Creates a new `IconManager` with a custom background worker pool size.
    pub fn new_with_workers(worker_count: usize) -> Self {
        let (req_tx, req_rx) = unbounded::<IconRequest>();
        let (upd_tx, upd_rx) = unbounded::<IconUpdate>();

        let cache = Arc::new(RwLock::new(HashMap::new()));
        let in_flight = Arc::new(Mutex::new(HashSet::new()));
        let running = Arc::new(AtomicBool::new(true));

        let actual_workers = worker_count.max(1);
        let mut worker_threads = Vec::with_capacity(actual_workers);

        for _ in 0..actual_workers {
            let rx = req_rx.clone();
            let tx = upd_tx.clone();
            let c = Arc::clone(&cache);
            let inflight = Arc::clone(&in_flight);
            let run = Arc::clone(&running);

            let handle = thread::spawn(move || {
                while run.load(Ordering::Relaxed) {
                    let request = match rx.recv() {
                        Ok(req) => req,
                        Err(_) => break, // Channel closed
                    };

                    // Run the 3-tier fallback pipeline off the UI thread
                    let icon_opt = extract_icon_fallback(request.hwnd, &request.exe_path);

                    // Clear in-flight status
                    if let Ok(mut inflight_lock) = inflight.lock() {
                        inflight_lock.remove(&request.key);
                    }

                    if let Some(icon) = icon_opt {
                        // Populate in-memory cache
                        if let Ok(mut cache_lock) = c.write() {
                            cache_lock.insert(request.key.clone(), icon.clone());
                            // If key contains a class name, also cache under executable path alone
                            if !request.class_name.is_empty() {
                                let exe_only_key = make_cache_key(&request.exe_path, "");
                                cache_lock.entry(exe_only_key).or_insert_with(|| icon.clone());
                            }
                        }

                        // Deliver update message back to main thread
                        let update = IconUpdate {
                            hwnd: request.hwnd,
                            exe_path: request.exe_path,
                            class_name: request.class_name,
                            key: request.key,
                            icon,
                        };
                        let _ = tx.send(update);
                    }
                }
            });

            worker_threads.push(handle);
        }

        Self {
            cache,
            in_flight,
            request_sender: req_tx,
            update_receiver: upd_rx,
            running,
            _worker_threads: worker_threads,
        }
    }

    /// Queries the in-memory cache. If missing, dispatches an asynchronous background
    /// resolution request to the worker pool and returns `None`.
    pub fn get_icon_or_request(&self, hwnd: isize, exe_path: &str) -> Option<IconRgba> {
        let class_name = if hwnd > 0 {
            get_window_class_name(hwnd).unwrap_or_default()
        } else {
            String::new()
        };

        let key = make_cache_key(exe_path, &class_name);

        // Check primary cache key
        if let Ok(cache) = self.cache.read() {
            if let Some(icon) = cache.get(&key) {
                return Some(icon.clone());
            }
            // Check fallback executable-only key
            if !class_name.is_empty() {
                let exe_only_key = make_cache_key(exe_path, "");
                if let Some(icon) = cache.get(&exe_only_key) {
                    return Some(icon.clone());
                }
            }
        }

        if hwnd <= 0 && exe_path.trim().is_empty() {
            return None;
        }

        // Enqueue background request if not already in-flight
        if let Ok(mut inflight) = self.in_flight.lock() {
            if inflight.insert(key.clone()) {
                let req = IconRequest {
                    hwnd,
                    exe_path: exe_path.to_string(),
                    class_name,
                    key,
                };
                let _ = self.request_sender.send(req);
            }
        }

        None
    }

    /// Retrieves an icon directly from the in-memory cache without requesting background extraction.
    pub fn get_cached(&self, exe_path: &str, class_name: &str) -> Option<IconRgba> {
        let key = make_cache_key(exe_path, class_name);
        if let Ok(cache) = self.cache.read() {
            if let Some(icon) = cache.get(&key) {
                return Some(icon.clone());
            }
            if !class_name.is_empty() {
                let exe_only_key = make_cache_key(exe_path, "");
                return cache.get(&exe_only_key).cloned();
            }
        }
        None
    }

    /// Directly inserts an icon into the in-memory cache under a specified key.
    pub fn insert_cache(&self, key: &str, icon: IconRgba) {
        if let Ok(mut cache) = self.cache.write() {
            cache.insert(key.to_string(), icon);
        }
    }

    /// Non-blocking check for decoded icon updates delivered by background workers.
    pub fn try_recv_update(&self) -> Option<IconUpdate> {
        self.update_receiver.try_recv().ok()
    }

    /// Exposes a reference to the update receiver for UI event polling.
    pub fn receiver(&self) -> &Receiver<IconUpdate> {
        &self.update_receiver
    }

    /// Explicitly queues an icon resolution request off-thread.
    pub fn request_icon(&self, hwnd: isize, exe_path: &str) {
        let _ = self.get_icon_or_request(hwnd, exe_path);
    }

    /// Returns the number of cached icon bitmaps.
    pub fn cache_size(&self) -> usize {
        self.cache.read().map(|c| c.len()).unwrap_or(0)
    }

    /// Checks if a key is currently being processed by background workers.
    pub fn is_in_flight(&self, key: &str) -> bool {
        self.in_flight
            .lock()
            .map(|set| set.contains(key))
            .unwrap_or(false)
    }

    /// Clears the in-memory cache.
    pub fn clear_cache(&self) {
        if let Ok(mut cache) = self.cache.write() {
            cache.clear();
        }
    }
}

impl Default for IconManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for IconManager {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
    }
}
