# 07: Live DWM Window Hover Thumbnails

**What to build:** Hardware-accelerated, zero-copy live window thumbnail previews that appear when hovering over a taskbar item, allowing users to visually inspect and select from open window instances.

**Blocked by:** 06: Task Grouping & Window Activation Controls

**Status:** closed

- [x] Displays a preview popup window when hovering over a task item representing one or more open windows.
- [x] Connects to the Desktop Window Manager thumbnail composition API to stream live window contents into the preview area without taking CPU screenshots.
- [x] For grouped tasks with multiple windows, renders individual preview tiles for each open window instance with its window title and close button.
- [x] Clicking a specific thumbnail preview immediately focuses and restores that window instance.
- [x] Unregisters and releases the DWM thumbnail composition cleanly when the hover ends or the window closes.
