use std::io;
use std::process::Command;
use crate::config::PinnedItem;

pub use crate::shell::trigger_start_menu;

/// Parses a command line arguments string into individual argument tokens,
/// respecting single and double quotes.
pub fn parse_args(args_str: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut chars = args_str.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '\'' if !in_double_quote => {
                in_single_quote = !in_single_quote;
            }
            '"' if !in_single_quote => {
                in_double_quote = !in_double_quote;
            }
            '\\' if in_double_quote && chars.peek() == Some(&'"') => {
                chars.next();
                current.push('"');
            }
            ch if ch.is_whitespace() && !in_single_quote && !in_double_quote => {
                if !current.is_empty() {
                    args.push(current);
                    current = String::new();
                }
            }
            _ => {
                current.push(ch);
            }
        }
    }

    if !current.is_empty() {
        args.push(current);
    }

    args
}

/// Constructs a configured `std::process::Command` from a `PinnedItem`.
pub fn build_command(item: &PinnedItem) -> Command {
    let mut cmd = Command::new(&item.path);

    let parsed_args = parse_args(&item.args);
    if !parsed_args.is_empty() {
        cmd.args(parsed_args);
    }

    if let Some(ref dir) = item.working_dir {
        let trimmed = dir.trim();
        if !trimmed.is_empty() {
            cmd.current_dir(trimmed);
        }
    }

    cmd
}

/// Launches a pinned application process asynchronously.
///
/// Returns `Ok(())` if the process spawned successfully, or an `io::Error` if the executable
/// could not be located or failed to execute.
pub fn launch_app(item: &PinnedItem) -> Result<(), io::Error> {
    log::info!("Launching pinned application: {} ({})", item.title, item.path);
    let mut cmd = build_command(item);
    let _child = cmd.spawn()?;
    Ok(())
}
