# 3. Live Window Previews via Desktop Window Manager (DWM) Thumbnails

We decided to use the native Desktop Window Manager API (`DwmRegisterThumbnail` and `DwmUpdateThumbnailProperties`) to render hover window previews directly, instead of capturing window screenshots with GDI/DXGI. DWM compositing occurs entirely inside the GPU pipeline of the OS window manager, eliminating pixel buffer copying, bitmap allocations, and high CPU usage in our process.

