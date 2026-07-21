use crate::app::Raycast;

mod app;
mod design_system;
mod ipc;
mod prism;
#[cfg(target_os = "linux")]
mod tray;

/// When invoked as `iced_raycast --clip-record` (by the clipboard plugin's
/// `wl-paste --watch` watcher), read the new clipboard text from stdin, record
/// it into the history store, and exit without starting the UI. Returns whether
/// this was a record invocation.
fn handle_clip_record() -> bool {
    if !std::env::args().any(|arg| arg == "--clip-record") {
        return false;
    }
    use std::io::Read;
    let mut buffer = Vec::new();
    if std::io::stdin().read_to_end(&mut buffer).is_ok() {
        if let Ok(text) = String::from_utf8(buffer) {
            core::record_clipboard(&text);
        }
    }
    true
}

#[cfg(not(target_os = "linux"))]
pub fn main() -> iced::Result {
    // Background clipboard recorder invocation — never touches the UI.
    if handle_clip_record() {
        return Ok(());
    }

    // Warm path: if a resident agent is already running, ask it to show the
    // launcher and exit immediately (instant open, nothing re-loaded).
    if ipc::try_show() {
        return Ok(());
    }

    // Cold path: become the resident agent. `iced::daemon` stays alive with no
    // window; the launcher window is created on demand — and, on this first
    // launch, immediately (see Raycast::new). Bind an OS hotkey to run the
    // binary to open it warm thereafter.
    iced::daemon(Raycast::new, Raycast::update, Raycast::view)
        .title(|_state, _id| String::from("Raycast"))
        .style(Raycast::style)
        .font(include_bytes!("../fonts/Roboto-Regular.ttf").as_slice())
        .font(include_bytes!("../fonts/Roboto-Medium.ttf").as_slice())
        .font(include_bytes!("../fonts/RobotoMono-Regular.ttf").as_slice())
        .subscription(Raycast::subscription)
        .run()
}

#[cfg(target_os = "linux")]
pub fn main() -> Result<(), iced_layershell::Error> {
    use iced_layershell::build_pattern::daemon;
    use iced_layershell::settings::{LayerShellSettings, Settings, StartMode};

    // Background clipboard recorder invocation — never touches the UI.
    if handle_clip_record() {
        return Ok(());
    }

    // Warm path: if a resident agent is already running, ask it to show the
    // launcher and exit immediately (instant open, nothing re-loaded).
    if ipc::try_show() {
        return Ok(());
    }

    // Cold path: become the resident agent. It runs in the background with NO
    // initial surface (StartMode::Background keeps the event loop alive with zero
    // windows); the launcher surface is created on demand — and, for this first
    // launch, immediately (see Raycast::new). The multi-window daemon also hosts
    // the Plugin Manager as a separate xdg_toplevel window.
    daemon(
        Raycast::new,
        Raycast::namespace,
        Raycast::update,
        Raycast::view,
    )
    .style(Raycast::style)
    .font(include_bytes!("../fonts/Roboto-Regular.ttf").as_slice())
    .font(include_bytes!("../fonts/Roboto-Medium.ttf").as_slice())
    .font(include_bytes!("../fonts/RobotoMono-Regular.ttf").as_slice())
    .subscription(Raycast::subscription)
    .settings(Settings {
        layer_settings: LayerShellSettings {
            start_mode: StartMode::Background,
            ..Default::default()
        },
        ..Default::default()
    })
    .run()
}
