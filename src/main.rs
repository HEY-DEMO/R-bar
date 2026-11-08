use std::env;
use std::rc::Rc;
use taskbar_clone::config::Config;
use taskbar_clone::launcher::launch_app;
use taskbar_clone::shell::{hide_native_taskbar, install_panic_watchdog, restore_native_taskbar, trigger_start_menu};

slint::include_modules!();

struct RestoreOnDrop;

impl Drop for RestoreOnDrop {
    fn drop(&mut self) {
        log::info!("Exiting: restoring native Windows taskbar...");
        restore_native_taskbar();
    }
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

    hide_native_taskbar();
    let _restore_guard = RestoreOnDrop;

    let cfg = Config::load_or_default();
    let app = MainWindow::new()?;

    app.set_is_vertical(cfg.is_vertical());

    // Wire Start button callback
    app.on_start_clicked(|| {
        log::info!("Start button clicked: triggering native Windows Start Menu");
        trigger_start_menu();
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

    // Wire pinned item click to launch application asynchronously
    let pinned_for_launch = pinned_configs.clone();
    app.on_pinned_clicked(move |idx| {
        if let Some(item) = pinned_for_launch.get(idx as usize) {
            log::info!("Pinned item clicked: {} ({})", item.title, item.path);
            if let Err(err) = launch_app(item) {
                log::error!("Failed to launch app {}: {}", item.title, err);
            }
        }
    });

    app.run()?;

    Ok(())
}
