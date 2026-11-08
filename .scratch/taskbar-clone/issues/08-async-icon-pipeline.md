# 08: Asynchronous Off-Thread Icon Pipeline

**What to build:** A non-blocking background icon extraction and caching subsystem that resolves high-resolution application icons without ever freezing or hitching the taskbar's UI rendering loop.

**Blocked by:** 05: Shell Hook Window Tracker & Qualification Filter

**Status:** ready-for-agent

- [ ] Extracts application icons using a fallback pipeline: window messages with timeout, window class icon handles, and executable PE resources.
- [ ] Offloads all icon querying, GDI bitmap decoding, and pixel conversions to a dedicated background worker pool.
- [ ] Maintains an in-memory cache of decoded RGBA icon buffers keyed by executable path and window class.
- [ ] Delivers extracted icon bitmaps back to the main UI thread via thread-safe message passing so newly opened applications update smoothly.
- [ ] Protects against unresponsive target windows by enforcing a bounded timeout on window icon messages.

