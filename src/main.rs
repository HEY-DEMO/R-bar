use std::env;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use taskbar_clone::appbar::MultiMonitorManager;
use taskbar_clone::config::Config;
use taskbar_clone::events::start_event_pump;
use taskbar_clone::launcher::launch_app;
use taskbar_clone::shell::{
    hide_native_taskbar, install_panic_watchdog, restore_native_taskbar, trigger_start_menu,
};
use taskbar_clone::system::{poll_system_status, set_master_volume, toggle_native_tray};
use taskbar_clone::tasks::{
    activate_window, enumerate_initial_windows, group_tasks, handle_task_click,
    update_tasks_on_event, GroupedTask, TaskItem,
};
use windows::Win32::Foundation::BOOL;
use windows::Win32::System::Console::SetConsoleCtrlHandler;

slint::include_modules!();

struct RestoreOnDrop {
    monitor_mgr: MultiMonitorManager,
}

impl Drop for RestoreOnDrop {
    fn drop(&mut self) {
        log::info!("Exiting: unregistering AppBars and restoring native Windows taskbar...");
        self.monitor_mgr.unregister_all();
        restore_native_taskbar();
    }
}

unsafe extern "system" fn console_ctrl_handler(_ctrl_type: u32) -> BOOL {
    log::info!("Console signal received: restoring native taskbar");
    restore_native_taskbar();
    BOOL(0)
}

fn sync_tasks_to_slint(groups: &[GroupedTask], app: &MainWindow) {
    let slint_tasks: Vec<TaskData> = groups
        .iter()
        .map(|g| {
            let initial = g
                .title
                .chars()
                .next()
                .map(|c| c.to_uppercase().to_string())
                .unwrap_or_else(|| "•".to_string());
            let active = g.is_active();
            let minimized = g.active_window().map(|w| w.is_minimized).unwrap_or(false);
            TaskData {
                title: g.title.clone().into(),
                initial: initial.into(),
                app_id: g.app_id.clone().into(),
                window_count: g.window_count() as i32,
                has_multiple: g.has_multiple_windows(),
                is_active: active,
                is_minimized: minimized,
            }
        })
        .collect();
    let model = Rc::new(slint::VecModel::from(slint_tasks));
    app.set_tasks(model.into());
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    if args.iter().any(|arg| arg == "--restore") {
        log::info!("CLI flag --restore detected. Restoring native taskbar and exiting.");
        println!("Restoring native Windows taskbar...");
        restore_native_taskbar();
        return Ok(());
    }

    log::info!("Starting Taskbar Clone...");
    install_panic_watchdog();
    unsafe {
        let _ = SetConsoleCtrlHandler(Some(console_ctrl_handler), true);
    }

    hide_native_taskbar();

    let cfg = Config::load_or_default();
    let app = MainWindow::new()?;

    app.set_is_vertical(cfg.is_vertical());
    app.set_theme_name(cfg.theme.name().into());

    // Compute primary display AppBar dimensions and position
    let mon_rect = taskbar_clone::appbar::get_primary_monitor_rect();
    let bar_rect = taskbar_clone::geometry::calculate_appbar_rect(cfg.edge, cfg.size, mon_rect);
    let bar_w = bar_rect.width() as u32;
    let bar_h = bar_rect.height() as u32;

    app.set_bar_width(bar_w as f32);
    app.set_bar_height(bar_h as f32);
    app.window()
        .set_size(slint::PhysicalSize::new(bar_w, bar_h));
    app.window()
        .set_position(slint::PhysicalPosition::new(bar_rect.left, bar_rect.top));

    // Register Multi-Monitor AppBars
    let mut monitor_mgr = MultiMonitorManager::new(cfg.edge, cfg.size);
    monitor_mgr.spawn_bars();
    let _restore_guard = RestoreOnDrop { monitor_mgr };

    // Wire Start button callback
    app.on_start_clicked(|| {
        log::info!("Start button clicked: triggering native Windows Start Menu");
        trigger_start_menu();
    });

    // Wire Native Tray toggle
    app.on_tray_toggle_clicked(|| {
        log::info!("Tray toggle clicked: toggling native Windows notification tray");
        toggle_native_tray();
    });

    // Wire master audio volume change
    app.on_volume_changed(|vol_pct| {
        let level = (vol_pct / 100.0).clamp(0.0, 1.0);
        let _ = set_master_volume(level);
    });

    // Populate pinned launcher items
    let pinned_configs = cfg.pinned.clone();
    let slint_pinned: Vec<PinnedData> = pinned_configs
        .iter()
        .map(|item| {
            let initial = item
                .title
                .chars()
                .next()
                .map(|c| c.to_uppercase().to_string())
                .unwrap_or_else(|| "•".to_string());
            let (icon, has_icon) = if !item.icon.is_empty() {
                if let Ok(img) = slint::Image::load_from_path(std::path::Path::new(&item.icon)) {
                    (img, true)
                } else {
                    (slint::Image::default(), false)
                }
            } else {
                (slint::Image::default(), false)
            };
            PinnedData {
                title: item.title.clone().into(),
                initial: initial.into(),
                has_icon,
                icon,
            }
        })
        .collect();

    let model = Rc::new(slint::VecModel::from(slint_pinned));
    app.set_pinned_items(model.into());

    // Wire pinned item click
    let pinned_for_launch = pinned_configs.clone();
    app.on_pinned_clicked(move |idx| {
        if let Some(item) = pinned_for_launch.get(idx as usize) {
            log::info!("Pinned item clicked: {} ({})", item.title, item.path);
            if let Err(err) = launch_app(item) {
                log::error!("Failed to launch app {}: {}", item.title, err);
            }
        }
    });

    // Initialize running tasks
    let initial_windows = enumerate_initial_windows();
    let initial_groups = group_tasks(initial_windows.clone());
    sync_tasks_to_slint(&initial_groups, &app);

    let raw_tasks: Arc<Mutex<Vec<TaskItem>>> = Arc::new(Mutex::new(initial_windows));
    let grouped_tasks: Arc<Mutex<Vec<GroupedTask>>> = Arc::new(Mutex::new(initial_groups));

    // Wire task item clicking
    let grouped_for_clicks = Arc::clone(&grouped_tasks);
    app.on_task_clicked(move |idx| {
        let mut groups = grouped_for_clicks.lock().unwrap();
        if let Some(group) = groups.get_mut(idx as usize) {
            log::info!("Task clicked: {} ({})", group.title, group.app_id);
            let action = handle_task_click(group);
            match action {
                taskbar_clone::controls::TaskClickAction::Activated(hwnd) => {
                    activate_window(hwnd);
                }
                taskbar_clone::controls::TaskClickAction::Minimized(hwnd) => {
                    taskbar_clone::tasks::minimize_window(hwnd);
                }
                taskbar_clone::controls::TaskClickAction::None => {}
            }
        }
    });

    // Start background event pump listening for Win32 shell hooks
    let (tx, rx) = crossbeam_channel::unbounded();
    let _event_thread = start_event_pump(tx);

    let app_weak = app.as_weak();
    let raw_tasks_worker = Arc::clone(&raw_tasks);
    let grouped_tasks_worker = Arc::clone(&grouped_tasks);

    // Event pump dispatcher running on a background worker thread
    std::thread::spawn(move || {
        while let Ok(event) = rx.recv() {
            let mut tasks = raw_tasks_worker.lock().unwrap();
            update_tasks_on_event(&mut tasks, &event);
            let updated_groups = group_tasks(tasks.clone());
            {
                let mut stored_groups = grouped_tasks_worker.lock().unwrap();
                *stored_groups = updated_groups.clone();
            }

            let app_weak_clone = app_weak.clone();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(app) = app_weak_clone.upgrade() {
                    sync_tasks_to_slint(&updated_groups, &app);
                }
            });
        }
    });

    // Periodic timer (1 second) for clock, date, battery, and audio volume
    let status_timer = slint::Timer::default();
    let app_status_weak = app.as_weak();
    status_timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(1000),
        move || {
            if let Some(app) = app_status_weak.upgrade() {
                let status = poll_system_status();
                app.set_time_text(status.time_str.into());
                app.set_date_text(status.date_str.into());
                if let Some(pct) = status.battery_percent {
                    app.set_battery_text(
                        format!("{}%{}", pct, if status.is_charging { " ⚡" } else { "" }).into(),
                    );
                } else {
                    app.set_battery_text("".into());
                }
                app.set_volume_percent(status.volume_percent as i32);
            }
        },
    );

    // Initial system status poll
    let initial_status = poll_system_status();
    app.set_time_text(initial_status.time_str.into());
    app.set_date_text(initial_status.date_str.into());
    if let Some(pct) = initial_status.battery_percent {
        app.set_battery_text(
            format!("{}%{}", pct, if initial_status.is_charging { " ⚡" } else { "" }).into(),
        );
    }
    app.set_volume_percent(initial_status.volume_percent as i32);

    app.run()?;

    Ok(())
}
