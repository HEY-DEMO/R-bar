use std::env;
use taskbar_clone::shell::{hide_native_taskbar, install_panic_watchdog, restore_native_taskbar};

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

    let app = MainWindow::new()?;
    app.run()?;

    Ok(())
}

