//! System-tray (StatusNotifierItem) icon for the resident agent.
//!
//! Makes the always-running agent visible and controllable instead of a hidden
//! background process. Uses `ksni` (pure-Rust SNI over D-Bus, no GTK); the
//! blocking backend runs the tray on its own threads. Menu actions that concern
//! the running app (Open Launcher, Quit) are forwarded into the iced event loop
//! as [`Message`]s through a subscription channel; the clipboard actions call
//! `core` directly.
//!
//! Requires a StatusNotifier host on the desktop (e.g. waybar's `tray` module)
//! to actually display.

use iced::Subscription;
use iced::futures::channel::mpsc::Sender;

use crate::app::Message;

/// The subscription that hosts the tray icon and forwards its menu events.
pub fn subscription() -> Subscription<Message> {
    Subscription::run(tray_stream)
}

fn tray_stream() -> impl iced::futures::Stream<Item = Message> {
    iced::stream::channel(8, |output: Sender<Message>| async move {
        use iced::futures::{SinkExt, StreamExt};

        // ksni's blocking `spawn` sets up the service (on its own threads) and
        // returns a handle we must keep alive. Menu callbacks push app-level
        // messages into this channel.
        let (events, mut receiver) = iced::futures::channel::mpsc::channel::<Message>(8);
        std::thread::spawn(move || {
            use ksni::blocking::TrayMethods;
            let tray = LauncherTray { events };
            match tray.assume_sni_available(true).spawn() {
                Ok(handle) => {
                    // Keep the service alive for the process lifetime.
                    while !handle.is_closed() {
                        std::thread::park();
                    }
                }
                Err(error) => eprintln!("tray: failed to start: {error}"),
            }
        });

        let mut output = output;
        while let Some(message) = receiver.next().await {
            let _ = output.send(message).await;
        }
    })
}

/// The tray icon and its menu. Holds a sender for the actions that must reach
/// the iced app.
struct LauncherTray {
    events: Sender<Message>,
}

impl ksni::Tray for LauncherTray {
    fn id(&self) -> String {
        "iced_raycast".into()
    }

    fn title(&self) -> String {
        "Raycast Launcher".into()
    }

    fn icon_name(&self) -> String {
        // A themed icon name; falls back gracefully if the theme lacks it.
        "system-search".into()
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        ksni::ToolTip {
            title: "Raycast Launcher".into(),
            description: "Running — clipboard history is being recorded".into(),
            icon_name: "system-search".into(),
            icon_pixmap: Vec::new(),
        }
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::{CheckmarkItem, MenuItem, StandardItem};

        vec![
            StandardItem {
                label: "Open Launcher".into(),
                icon_name: "system-search".into(),
                activate: Box::new(|tray: &mut Self| {
                    let _ = tray.events.try_send(Message::Show);
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            CheckmarkItem {
                label: "Record clipboard".into(),
                // Read live so the checkmark reflects the current setting even if
                // it was toggled from the Plugin Manager.
                checked: core::clipboard_recording_enabled(),
                activate: Box::new(|_tray: &mut Self| {
                    core::set_clipboard_recording(!core::clipboard_recording_enabled());
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Clear clipboard history".into(),
                icon_name: "edit-clear".into(),
                activate: Box::new(|_tray: &mut Self| core::clear_clipboard_history()),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Quit".into(),
                icon_name: "application-exit".into(),
                activate: Box::new(|tray: &mut Self| {
                    let _ = tray.events.try_send(Message::QuitAgent);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}
