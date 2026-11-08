# 08: Asynchronous Off-Thread Icon Pipeline

**What to build:** A non-blocking background icon extraction and caching subsystem that resolves high-resolution application icons without ever freezing or hitching the taskbar's UI rendering loop.

**Blocked by:** 05: Shell Hook Window Tracker & Qualification Filter

**Status:** closed

- [x] Extracts application icons using a fallback pipeline: window messages with timeout, window class icon handles, and executable PE resources.
- [x] Offloads all icon querying, GDI bitmap decoding, and pixel conversions to a dedicated background worker pool.
- [x] Maintains an in-memory cache of decoded RGBA icon buffers keyed by executable path and window class.
- [x] Delivers extracted icon bitmaps back to the main UI thread via thread-safe message passing so newly opened applications update smoothly.
- [x] Protects against unresponsive target windows by enforcing a bounded timeout on window icon messages.

## Comments

### Resolution
- Merged branch `ticket-08` into `feat/taskbar-clone` at commit `17ed0b5` (source commit `d365fca`).
- Implemented `src/icons.rs` with the following architectural components:
  - `IconRgba`: 32-bit RGBA pixel buffer struct storing width, height, and raw bytes with helper methods.
  - `IconCacheKey`: Cache key hierarchy keyed by executable path and window class name.
  - Multi-tiered icon extraction fallback pipeline:
    1. Window messages (`WM_GETICON` with `ICON_BIG`, `ICON_SMALL2`, `ICON_SMALL`) using `SendMessageTimeoutW` bounded strictly to 150ms (`ICON_MESSAGE_TIMEOUT_MS`) with `SMTO_ABORTIFHUNG | SMTO_BLOCK` flags to prevent unresponsive targets from freezing execution.
    2. Window class icon handles via `GetClassLongPtrW` (`GCLP_HICON`, `GCLP_HICONSM`).
    3. Executable PE resources via `ExtractIconExW`.
    4. Procedural fallback placeholder icon generation (`create_fallback_icon`) ensuring every task item always has a valid visual representation.
  - GDI bitmap conversion (`icon_to_rgba`): Safe extraction of icon mask/color bitmaps via `GetIconInfo`, `GetObjectW`, and `GetDIBits` conversion to 32-bit RGBA pixels with proper Win32 resource deallocation (`DeleteObject`, `DeleteDC`, `DestroyIcon`).
  - `IconManager`: Asynchronous background worker pool managing icon requests via thread-safe `crossbeam-channel` queues (`Sender<IconRequest>`, `Receiver<IconUpdate>`), thread-safe in-memory cache (`RwLock<HashMap<IconCacheKey, Arc<IconRgba>>>`), and deduplicated pending request tracking.
- Integration tests in `tests/icons_test.rs`:
  - `test_bounded_timeout_constant`
  - `test_icon_rgba_pixel_buffer`
  - `test_cache_key_generation`
  - `test_create_fallback_icon`
  - `test_timeout_resilience_on_invalid_window`
  - `test_window_class_icon_invalid_hwnd`
  - `test_icon_extraction_from_pe_resource`
  - `test_fallback_pipeline`
  - `test_icon_manager_cache_storage_and_retrieval`
  - `test_icon_manager_async_request_and_delivery`
- Verification:
  - `cargo test`: All 69 tests pass across 11 test suites (10/10 icon tests pass).
  - `cargo clippy --all-targets`: Passed with 0 warnings.
