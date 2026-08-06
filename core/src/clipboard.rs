//! Clipboard access that survives the launcher exiting.
//!
//! The app closes immediately after acting on a result, so an in-process
//! clipboard write (which only owns the selection while the process lives) is
//! lost on Wayland the moment we exit. Instead we hand the text to a small
//! external helper that keeps serving the selection after we're gone:
//! `wl-copy` (Wayland) / `xclip` / `xsel` on Linux, `pbcopy` on macOS, `clip`
//! on Windows.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, anyhow};

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

/// Read the current text contents of the system clipboard, or `None` if it is
/// empty, non-text, or no helper is available.
pub fn paste() -> Option<String> {
    for (program, args) in paste_candidates() {
        if let Some(text) = spawn_paste(program, args) {
            return Some(text);
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn paste_candidates() -> Vec<(&'static str, &'static [&'static str])> {
    let wl_paste: (&str, &[&str]) = ("wl-paste", &["--no-newline"]);
    let xclip: (&str, &[&str]) = ("xclip", &["-selection", "clipboard", "-o"]);
    let xsel: (&str, &[&str]) = ("xsel", &["--clipboard", "--output"]);

    if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        vec![wl_paste, xclip, xsel]
    } else {
        vec![xclip, xsel, wl_paste]
    }
}

#[cfg(target_os = "macos")]
fn paste_candidates() -> Vec<(&'static str, &'static [&'static str])> {
    vec![("pbpaste", &[])]
}

#[cfg(target_os = "windows")]
fn paste_candidates() -> Vec<(&'static str, &'static [&'static str])> {
    vec![("powershell", &["-NoProfile", "-Command", "Get-Clipboard"])]
}

/// Run a paste helper and return its stdout as text, or `None` if it failed,
/// produced non-UTF-8 (e.g. an image), or was empty.
fn spawn_paste(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8(output.stdout).ok()?;
    (!text.is_empty()).then_some(text)
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
        assert!(
            pasted.contains(marker),
            "clipboard missing marker, got {pasted:?}"
        );
    }
}

/// Copy raw image `bytes` to the system clipboard under `mime` (e.g.
/// `"image/gif"`), so paste-aware apps receive the image itself rather than a
/// URL string. Bytes are staged in a temp file so the platform helper can read
/// them; the file is left in place because some helpers (macOS `osascript`,
/// Linux `wl-copy` / `xclip`) serve the clipboard from that path asynchronously
/// after we exit.
pub fn copy_image(bytes: &[u8], mime: &str) -> Result<()> {
    let path = write_temp(bytes, extension_for(mime))?;
    copy_image_from_path(&path, mime)
}

fn extension_for(mime: &str) -> &'static str {
    match mime {
        "image/gif" => "gif",
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/webp" => "webp",
        "image/bmp" => "bmp",
        "image/tiff" => "tiff",
        _ => "bin",
    }
}

fn write_temp(bytes: &[u8], extension: &str) -> Result<PathBuf> {
    use std::fs::File;
    use std::time::{SystemTime, UNIX_EPOCH};

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let path = std::env::temp_dir().join(format!("iced_raycast_clip_{stamp}.{extension}"));

    let mut file = File::create(&path).with_context(|| format!("create {}", path.display()))?;
    file.write_all(bytes)
        .with_context(|| format!("write {}", path.display()))?;
    Ok(path)
}

#[cfg(target_os = "macos")]
fn copy_image_from_path(path: &std::path::Path, _mime: &str) -> Result<()> {
    // Put ONLY a file-URL on the pasteboard (mirroring Finder's "Copy file"),
    // not the raw image bytes. Rationale: apps like Microsoft Teams, iMessage,
    // and Electron-based chat clients receive Chromium's clipboard, which
    // prefers `public.tiff` / `public.png` when image data is present — and
    // macOS auto-derives those from GIF bytes as *static* frames, so animation
    // is always lost on paste. A file-URL sidesteps the type-promotion pipeline
    // entirely: the receiving app attaches the .gif as a real file and plays
    // it back animated.
    let path_str = path.to_str().ok_or_else(|| anyhow!("non-utf8 temp path"))?;

    let script = format!(
        r#"ObjC.import("AppKit");
var url = $.NSURL.fileURLWithPath({path});
var pb = $.NSPasteboard.generalPasteboard;
pb.clearContents;
pb.writeObjects($.NSArray.arrayWithObject(url));
"#,
        path = js_string(path_str),
    );

    let status = Command::new("osascript")
        .args(["-l", "JavaScript", "-e", &script])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .context("spawn osascript")?;

    if !status.success() {
        return Err(anyhow!("osascript failed (exit {status})"));
    }
    Ok(())
}

/// Encode `value` as a JavaScript string literal (double-quoted, with control
/// characters escaped). Used to safely embed paths and UTIs in the JXA script.
#[cfg(target_os = "macos")]
fn js_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(target_os = "linux")]
fn copy_image_from_path(path: &std::path::Path, mime: &str) -> Result<()> {
    let bytes = std::fs::read(path).with_context(|| format!("read {}", path.display()))?;

    let attempts: [(&str, Vec<&str>); 2] = if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        [
            ("wl-copy", vec!["--type", mime]),
            ("xclip", vec!["-selection", "clipboard", "-t", mime]),
        ]
    } else {
        [
            ("xclip", vec!["-selection", "clipboard", "-t", mime]),
            ("wl-copy", vec!["--type", mime]),
        ]
    };

    let mut last_error: Option<anyhow::Error> = None;
    for (program, args) in attempts {
        match spawn_copy_bytes(program, &args, &bytes) {
            Ok(()) => return Ok(()),
            Err(error) => last_error = Some(error),
        }
    }

    Err(last_error.unwrap_or_else(|| {
        anyhow!("no image clipboard helper available (install wl-clipboard/xclip)")
    }))
}

#[cfg(target_os = "linux")]
fn spawn_copy_bytes(program: &str, args: &[&str], bytes: &[u8]) -> Result<()> {
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    use std::os::unix::process::CommandExt;
    command.process_group(0);

    let mut child = command.spawn()?;
    child
        .stdin
        .take()
        .ok_or_else(|| anyhow!("clipboard helper has no stdin"))?
        .write_all(bytes)?;

    Ok(())
}

#[cfg(target_os = "windows")]
fn copy_image_from_path(path: &std::path::Path, mime: &str) -> Result<()> {
    // Persist the GIF (or other image) to the clipboard as its native MIME
    // type via WPF's DataObject, so paste-aware apps receive real bytes.
    // `SetDataObject($data, $true)` copies data across the process boundary
    // so the clipboard survives PowerShell exiting.
    let path_str = path
        .to_str()
        .ok_or_else(|| anyhow!("non-utf8 temp path"))?
        .replace('\'', "''");
    let script = format!(
        "Add-Type -AssemblyName System.Windows.Forms; \
         $bytes = [System.IO.File]::ReadAllBytes('{path_str}'); \
         $ms = New-Object System.IO.MemoryStream(,$bytes); \
         $data = New-Object System.Windows.Forms.DataObject('{mime}', $ms); \
         [System.Windows.Forms.Clipboard]::SetDataObject($data, $true)"
    );

    let status = Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .context("spawn powershell")?;

    if !status.success() {
        return Err(anyhow!("powershell failed (exit {status})"));
    }
    Ok(())
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
