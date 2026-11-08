# 5. Asynchronous Off-Thread Icon Extraction Pipeline

We decided to process all application icon queries (`WM_GETICON`, `ExtractIconExW`, and UWP manifest parsing) in a dedicated background thread pool with an in-memory cache, rather than synchronously on the UI event thread. Windows icon resolution can trigger disk I/O, PE header parsing, or synchronous IPC messages to target windows. Doing this off-thread guarantees that hanging, slow, or crashed target applications never stall the taskbar's UI loop or animation framerate.

