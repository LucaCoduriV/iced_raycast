//! Open URLs (and files) with the platform's default handler.
//!
//! The launcher exits right after acting, so the opener is spawned detached and
//! we don't wait on it.

use std::process::{Command, Stdio};

use anyhow::{Result, anyhow};

/// Open `target` (a URL or path) in the default application.
pub fn url(target: &str) -> Result<()> {
    let mut last_error = None;

    for (program, args) in openers() {
        match spawn(program, args, target) {
            Ok(()) => return Ok(()),
            Err(error) => last_error = Some(error),
        }
    }

    Err(last_error.unwrap_or_else(|| anyhow!("no URL opener available")))
}

#[cfg(target_os = "linux")]
fn openers() -> Vec<(&'static str, &'static [&'static str])> {
    vec![("xdg-open", &[]), ("gio", &["open"])]
}

#[cfg(target_os = "macos")]
fn openers() -> Vec<(&'static str, &'static [&'static str])> {
    vec![("open", &[])]
}

#[cfg(target_os = "windows")]
fn openers() -> Vec<(&'static str, &'static [&'static str])> {
    // `cmd /c start "" <url>` opens with the default handler.
    vec![("cmd", &["/c", "start", ""])]
}

fn spawn(program: &str, args: &[&str], target: &str) -> Result<()> {
    let mut command = Command::new(program);
    command
        .args(args)
        .arg(target)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }

    command.spawn()?;
    Ok(())
}
