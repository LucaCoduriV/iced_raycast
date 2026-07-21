//! Clipboard access that survives the launcher exiting.
//!
//! The app closes immediately after acting on a result, so an in-process
//! clipboard write (which only owns the selection while the process lives) is
//! lost on Wayland the moment we exit. Instead we hand the text to a small
//! external helper that keeps serving the selection after we're gone:
//! `wl-copy` (Wayland) / `xclip` / `xsel` on Linux, `pbcopy` on macOS, `clip`
//! on Windows.

use std::io::Write;
use std::process::{Command, Stdio};

use anyhow::{Result, anyhow};

/// Copy `text` to the system clipboard via the first available helper.
pub fn copy(text: &str) -> Result<()> {
    let mut last_error = None;

    for (program, args) in candidates() {
        match spawn_copy(program, args, text) {
            Ok(()) => return Ok(()),
            Err(error) => last_error = Some(error),
        }
    }

    Err(last_error
        .unwrap_or_else(|| anyhow!("no clipboard helper available (install wl-clipboard/xclip)")))
}

#[cfg(target_os = "linux")]
fn candidates() -> Vec<(&'static str, &'static [&'static str])> {
    let wl_copy: (&str, &[&str]) = ("wl-copy", &[]);
    let xclip: (&str, &[&str]) = ("xclip", &["-selection", "clipboard"]);
    let xsel: (&str, &[&str]) = ("xsel", &["--clipboard", "--input"]);

    // Prefer the helper matching the session; keep the others as fallbacks.
    if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        vec![wl_copy, xclip, xsel]
    } else {
        vec![xclip, xsel, wl_copy]
    }
}

#[cfg(target_os = "macos")]
fn candidates() -> Vec<(&'static str, &'static [&'static str])> {
    vec![("pbcopy", &[])]
}

#[cfg(target_os = "windows")]
fn candidates() -> Vec<(&'static str, &'static [&'static str])> {
    vec![("clip", &[])]
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    /// Round-trips text through the real clipboard helper. Opt-in (it clobbers
    /// the session clipboard): set `ICED_RAYCAST_TEST_CLIPBOARD`.
    #[test]
    fn copies_and_persists() {
        if std::env::var_os("ICED_RAYCAST_TEST_CLIPBOARD").is_none() {
            eprintln!("skipped: set ICED_RAYCAST_TEST_CLIPBOARD to run");
            return;
        }

        let marker = "iced_raycast_clipboard_marker_42";
        copy(marker).expect("copy failed");
        std::thread::sleep(std::time::Duration::from_millis(200));

        let output = Command::new("wl-paste").output().expect("wl-paste failed");
        let pasted = String::from_utf8_lossy(&output.stdout);
        assert!(pasted.contains(marker), "clipboard missing marker, got {pasted:?}");
    }
}

fn spawn_copy(program: &str, args: &[&str], text: &str) -> Result<()> {
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    // Detach so the helper isn't torn down when the launcher exits.
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }

    let mut child = command.spawn()?;

    child
        .stdin
        .take()
        .ok_or_else(|| anyhow!("clipboard helper has no stdin"))?
        .write_all(text.as_bytes())?;

    // Don't wait: wl-copy/xclip/xsel background themselves to keep serving the
    // selection. Dropping the handle leaves them running.
    Ok(())
}
