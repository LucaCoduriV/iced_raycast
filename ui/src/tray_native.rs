//! Native system tray + global hotkey for the resident agent on Windows/macOS.
//!
//! The Linux build gets its tray from `ksni` (pure-Rust SNI) and its hotkey from
//! the compositor keybind that runs the binary. Windows and macOS have neither,
//! so this module wires up the platform-native equivalents:
//!
//! * **Tray** — `tray-icon`, which wraps `Shell_NotifyIcon` on Windows and
//!   `NSStatusItem` on macOS. Menu: Open Launcher / Clear Clipboard History / Quit.
//! * **Global hotkey** — `global-hotkey`, which wraps `RegisterHotKey` on Windows
//!   and Carbon's `RegisterEventHotKey` on macOS. Default: **Alt/Option + Space**.
//!
//! ## Threading
//!
//! Both crates have strict thread rules: the tray and the hotkey manager must be
//! created on the thread running the event loop (the *main* thread on macOS), and
//! on macOS the loop must already be *running* — the earliest safe point is
//! winit's `StartCause::Init`. iced hides its winit event loop, so we can't hook
//! that init callback directly. Instead [`ensure_installed`] is called from the
//! app's `update`, which iced always runs on the main thread with the loop
//! already spinning; a [`std::sync::Once`] makes it a one-time setup. The created
//! objects are leaked (`mem::forget`) so they live for the resident process — we
//! never want to drop them (dropping would remove the icon / unregister the key).
//!
//! Their events arrive on the crates' global, cross-thread channels, which
//! [`subscription`] drains from a helper thread and forwards into the iced event
//! loop as [`Message`]s — the same shape as the Linux `tray` module.

use std::sync::{Once, OnceLock};

use iced::Subscription;
use iced::futures::channel::mpsc::Sender;

use tray_icon::menu::MenuId;

use crate::app::Message;

/// Menu-item ids, captured at install time so the event thread can tell which
/// item fired (muda identifies menu events only by id).
static SHOW_ID: OnceLock<MenuId> = OnceLock::new();
static CLEAR_ID: OnceLock<MenuId> = OnceLock::new();
static QUIT_ID: OnceLock<MenuId> = OnceLock::new();

/// Create the tray icon and register the global hotkey, exactly once.
///
/// MUST be called on the main thread with the event loop running — call it from
/// `Raycast::update` (see the module docs for why). Safe to call on every update;
/// the [`Once`] collapses all but the first to a cheap atomic load.
pub fn ensure_installed() {
    static INSTALL: Once = Once::new();
    INSTALL.call_once(|| {
        if let Err(error) = install() {
            eprintln!("tray/hotkey: failed to initialise: {error}");
        }
    });
}

fn install() -> anyhow::Result<()> {
    use tray_icon::TrayIconBuilder;
    use tray_icon::menu::{Menu, MenuItem, PredefinedMenuItem};

    // Build the menu and remember each item's id for event routing.
    let menu = Menu::new();
    let show = MenuItem::new("Open Launcher", true, None);
    let clear = MenuItem::new("Clear Clipboard History", true, None);
    let quit = MenuItem::new("Quit", true, None);
    menu.append(&show)?;
    menu.append(&PredefinedMenuItem::separator())?;
    menu.append(&clear)?;
    menu.append(&PredefinedMenuItem::separator())?;
    menu.append(&quit)?;
    let _ = SHOW_ID.set(show.id().clone());
    let _ = CLEAR_ID.set(clear.id().clone());
    let _ = QUIT_ID.set(quit.id().clone());

    // Build the tray icon. Leak it: dropping a `TrayIcon` removes the icon, and
    // this one must live for the whole resident-process lifetime.
    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("Raycast Launcher")
        .with_icon(load_icon()?)
        .build()?;
    std::mem::forget(tray);

    // Register the global hotkey. Leak the manager for the same reason (its
    // `Drop` unregisters the key).
    use global_hotkey::GlobalHotKeyManager;
    use global_hotkey::hotkey::{Code, HotKey, Modifiers};
    let manager = GlobalHotKeyManager::new()?;
    manager.register(HotKey::new(Some(Modifiers::ALT), Code::Space))?;
    std::mem::forget(manager);

    Ok(())
}

/// Decode the bundled placeholder PNG into an RGBA tray icon.
fn load_icon() -> anyhow::Result<tray_icon::Icon> {
    let bytes = include_bytes!("../../assets/icon_placeholder.png");
    let image = image::load_from_memory(bytes)?.into_rgba8();
    let (width, height) = image.dimensions();
    Ok(tray_icon::Icon::from_rgba(image.into_raw(), width, height)?)
}

/// The subscription that drains the tray-menu and global-hotkey event channels
/// and forwards them as app [`Message`]s.
pub fn subscription() -> Subscription<Message> {
    Subscription::run(event_stream)
}

fn event_stream() -> impl iced::futures::Stream<Item = Message> {
    iced::stream::channel(16, |output: Sender<Message>| async move {
        // `try_send` doesn't need the async executor, so — like the Linux tray
        // and IPC subscriptions — we move the sender into plain threads that push
        // messages straight in. The crates' receivers are crossbeam channels, so
        // each thread *blocks* on `recv()`: the event is forwarded the instant it
        // arrives, with none of the latency a poll-and-sleep loop would add.

        // Global-hotkey presses show the launcher (ignore the release half).
        let mut hotkey_sender = output.clone();
        std::thread::spawn(move || {
            use global_hotkey::{GlobalHotKeyEvent, HotKeyState};
            let hotkeys = GlobalHotKeyEvent::receiver();
            while let Ok(event) = hotkeys.recv() {
                if event.state == HotKeyState::Pressed {
                    let _ = hotkey_sender.try_send(Message::Show);
                }
            }
        });

        // Tray-menu clicks, matched by the ids captured at install time.
        let mut menu_sender = output;
        std::thread::spawn(move || {
            use tray_icon::menu::MenuEvent;
            let menu = MenuEvent::receiver();
            while let Ok(event) = menu.recv() {
                if Some(&event.id) == SHOW_ID.get() {
                    let _ = menu_sender.try_send(Message::Show);
                } else if Some(&event.id) == QUIT_ID.get() {
                    let _ = menu_sender.try_send(Message::QuitAgent);
                } else if Some(&event.id) == CLEAR_ID.get() {
                    // Pure `core` action, like the Linux tray — no round-trip
                    // through the iced event loop needed.
                    core::clear_clipboard_history();
                }
            }
        });

        // Keep the stream alive for the lifetime of the app.
        std::future::pending::<()>().await;
    })
}
