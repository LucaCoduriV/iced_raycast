use core::AppState;

use iced::{Color, Element, Event, Task, event, widget::container};
#[cfg(target_os = "linux")]
use iced_layershell::actions::IcedXdgWindowSettings;
#[cfg(target_os = "linux")]
use iced_layershell::reexport::Layer;
#[cfg(target_os = "linux")]
use iced_layershell::to_layer_message;

use crate::prism;
use crate::prism::PrismEvent;

/// Initial size of the Plugin Manager window (a normal xdg_toplevel on Linux).
#[cfg(target_os = "linux")]
const SETTINGS_SIZE: (u32, u32) = (900, 620);
/// Size of the launcher surface / window.
const LAUNCHER_SIZE: (u32, u32) = (700, 500);

pub struct Raycast {
    prism: prism::Prism,
    app_state: AppState,
    /// The launcher window/surface while it is shown. The app is a resident
    /// process: the window is created on "show" and destroyed on close, but the
    /// process (and warm registry) live on.
    launcher: Option<iced::window::Id>,
    /// The Plugin Manager window while it is open (a separate xdg_toplevel on
    /// Linux; elsewhere the manager renders inline in the launcher window).
    #[cfg(target_os = "linux")]
    settings_window: Option<iced::window::Id>,
    /// The `wl-paste --watch` clipboard recorder, owned so it stops when the
    /// agent quits (and dies with the agent via `PR_SET_PDEATHSIG` on a crash).
    #[cfg(target_os = "linux")]
    clipboard_watcher: Option<std::process::Child>,
}

impl Raycast {
    pub fn new() -> (Raycast, Task<Message>) {
        let app_state = AppState::load();
        let (prism, prism_task) = prism::Prism::new();
        // Apply the user's saved preferences to the freshly-loaded plugins.
        prism.hydrate_preferences(&app_state);

        let state = Raycast {
            prism,
            app_state,
            launcher: None,
            #[cfg(target_os = "linux")]
            settings_window: None,
            // Start the continuous clipboard recorder, owned by this agent.
            #[cfg(target_os = "linux")]
            clipboard_watcher: spawn_clipboard_watcher(),
        };

        // The process boots resident (no window); show the launcher immediately
        // for this first invocation.
        let boot = Task::batch([
            prism_task.map(Message::PrismEvent),
            Task::done(Message::Show),
        ]);

        (state, boot)
    }

    pub fn namespace() -> String {
        String::from("RaycastClone")
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::PrismEvent(prism_event) => self.handle_prism_event(prism_event),
            Message::Run => {
                self.launch_selected();
                self.close_launcher()
            }
            Message::ExitApp => self.close_launcher(),
            Message::Show => self.show_launcher(),
            Message::QuitAgent => self.quit_agent(),
            Message::WindowClosed(id) => self.on_window_closed(id),
            _ => Task::none(),
        }
    }

    /// Launch the selected application/command (the old `Run` body).
    fn launch_selected(&mut self) {
        if let Some(entry) = self.prism.get_selected_entry().cloned() {
            // Plugin-result effects (copy / push view / close) are handled inside
            // Prism; this path only launches apps and commands.
            self.app_state.record_usage(&entry.entry.entity);

            let argument = self.prism.get_argument();
            if let Some(arg) = argument.as_deref().filter(|a| !a.is_empty()) {
                self.app_state.record_argument(entry.entry.name(), arg);
            }

            if let Err(e) = self.app_state.save() {
                eprintln!("Failed to save state: {}", e);
            }

            if let Err(e) = entry.entry.execute(argument) {
                eprintln!("Failed to launch: {}", e);
            }
        }
    }

    /// Show the launcher: create its window if it isn't already visible, reset
    /// the (warm) launcher state, and focus the search box.
    fn show_launcher(&mut self) -> Task<Message> {
        if self.launcher.is_some() {
            return Task::none(); // already visible
        }
        self.prism.reset();

        #[cfg(target_os = "linux")]
        {
            let (id, open) = Message::layershell_open(launcher_layer_settings());
            self.launcher = Some(id);
            let focus = self.prism.focus_search().map(Message::PrismEvent);
            Task::batch([open, focus])
        }
        #[cfg(not(target_os = "linux"))]
        {
            let (id, open) = iced::window::open(launcher_window_settings());
            self.launcher = Some(id);
            // Once the window exists, focus it and its search input (Initialized
            // focuses the search).
            Task::batch([
                open.map(|_| Message::PrismEvent(PrismEvent::Initialized)),
                iced::window::gain_focus(id),
            ])
        }
    }

    /// Close the launcher after acting on it. This hides the window but keeps the
    /// resident process alive.
    fn close_launcher(&mut self) -> Task<Message> {
        let Some(id) = self.launcher.take() else {
            return Task::none();
        };
        #[cfg(target_os = "linux")]
        {
            Task::done(Message::RemoveWindow(id))
        }
        #[cfg(not(target_os = "linux"))]
        {
            iced::window::close(id)
        }
    }

    /// Quit the resident agent entirely (e.g. from the tray).
    fn quit_agent(&mut self) -> Task<Message> {
        #[cfg(target_os = "linux")]
        if let Some(mut child) = self.clipboard_watcher.take() {
            // Stop recording the clipboard when the agent quits.
            let _ = child.kill();
        }
        iced::exit()
    }

    /// A window was closed: drop our reference so the next "show" recreates it.
    fn on_window_closed(&mut self, id: iced::window::Id) -> Task<Message> {
        if Some(id) == self.launcher {
            self.launcher = None;
            return Task::none();
        }
        #[cfg(target_os = "linux")]
        if Some(id) == self.settings_window {
            self.settings_window = None;
            self.prism.close_plugin_manager();
            return self.set_launcher_layer(Layer::Top);
        }
        Task::none()
    }

    /// Route a Prism event, managing the Plugin Manager window on Linux.
    #[cfg(target_os = "linux")]
    fn handle_prism_event(&mut self, prism_event: PrismEvent) -> Task<Message> {
        let was_open = self.prism.is_plugin_manager_open();
        let task = self
            .prism
            .update(prism_event, &mut self.app_state)
            .map(map_prism_event);
        let now_open = self.prism.is_plugin_manager_open();

        if now_open && !was_open && self.settings_window.is_none() {
            // Open the manager as a normal window and lower the launcher beneath
            // it (a Top layer surface would otherwise cover it).
            let (id, open) = Message::base_window_open(IcedXdgWindowSettings {
                size: Some(SETTINGS_SIZE),
                client_side_decorations: true,
            });
            self.settings_window = Some(id);
            return Task::batch([task, open, self.set_launcher_layer(Layer::Background)]);
        }
        if !now_open && was_open {
            return Task::batch([
                task,
                self.close_settings_window(),
                self.set_launcher_layer(Layer::Top),
            ]);
        }
        task
    }

    #[cfg(not(target_os = "linux"))]
    fn handle_prism_event(&mut self, prism_event: PrismEvent) -> Task<Message> {
        self.prism
            .update(prism_event, &mut self.app_state)
            .map(map_prism_event)
    }

    /// Close the Plugin Manager window if one is open.
    #[cfg(target_os = "linux")]
    fn close_settings_window(&mut self) -> Task<Message> {
        match self.settings_window.take() {
            Some(id) => Task::done(Message::RemoveWindow(id)),
            None => Task::none(),
        }
    }

    /// Change the launcher surface's layer, if it's shown. Used to tuck it below
    /// the manager window and restore it afterwards.
    #[cfg(target_os = "linux")]
    fn set_launcher_layer(&self, layer: Layer) -> Task<Message> {
        match self.launcher {
            Some(id) => Task::done(Message::LayerChange { id, layer }),
            None => Task::none(),
        }
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        use iced::Subscription;

        let mut subscriptions = vec![
            event::listen().map(Message::IcedEvent),
            self.prism.subscription().map(|event| match event {
                PrismEvent::ExitApp => Message::ExitApp,
                _ => Message::PrismEvent(event),
            }),
            // Resident-agent IPC "show" trigger + window-close notifications.
            Subscription::run(show_stream),
            iced::window::close_events().map(Message::WindowClosed),
        ];

        // The system-tray icon (Linux; ksni). Native trays for Windows/macOS are
        // still TODO.
        #[cfg(target_os = "linux")]
        subscriptions.push(crate::tray::subscription());

        Subscription::batch(subscriptions)
    }

    /// Multi-window view: the launcher window, plus (on Linux) the separate
    /// Plugin Manager window. Elsewhere the manager renders inline in the
    /// launcher window.
    pub fn view(&self, id: iced::window::Id) -> Element<'_, Message> {
        #[cfg(target_os = "linux")]
        if Some(id) == self.settings_window {
            return match self.prism.plugin_manager_view() {
                Some(view) => view.map(Message::PrismEvent),
                None => container("").into(),
            };
        }

        #[cfg(target_os = "linux")]
        let content = self.prism.view();
        #[cfg(not(target_os = "linux"))]
        let content = match self.prism.plugin_manager_view() {
            Some(view) => view,
            None => self.prism.view(),
        };

        let _ = id;
        container(content.map(Message::PrismEvent)).into()
    }

    pub fn style(&self, _theme: &iced::Theme) -> iced::theme::Style {
        iced::theme::Style {
            background_color: Color::TRANSPARENT,
            text_color: Color::WHITE,
        }
    }
}

/// Map events bubbled up from Prism onto the app's own messages.
fn map_prism_event(event: PrismEvent) -> Message {
    match event {
        PrismEvent::Run => Message::Run,
        PrismEvent::ExitApp => Message::ExitApp,
        e => Message::PrismEvent(e),
    }
}

/// Spawn the continuous clipboard recorder (`wl-paste --watch <self> --clip-record`)
/// as a child of the agent. `PR_SET_PDEATHSIG` makes it receive SIGTERM if the
/// agent dies for any reason (crash included), and it's killed explicitly on a
/// clean Quit — so recording never outlives the agent. Wayland only; other
/// environments fall back to capture-on-open.
#[cfg(target_os = "linux")]
fn spawn_clipboard_watcher() -> Option<std::process::Child> {
    use std::os::unix::process::CommandExt;
    use std::process::{Command, Stdio};

    if std::env::var_os("WAYLAND_DISPLAY").is_none() {
        return None;
    }
    let exe = std::env::current_exe().ok()?;

    let mut command = Command::new("wl-paste");
    command
        .arg("--watch")
        .arg(&exe)
        .arg("--clip-record")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    // SAFETY: only async-signal-safe calls (prctl/getppid/_exit) run in the
    // child between fork and exec.
    unsafe {
        command.pre_exec(|| {
            libc::prctl(
                libc::PR_SET_PDEATHSIG,
                libc::SIGTERM as libc::c_ulong,
                0,
                0,
                0,
            );
            // Guard the race where the parent already exited before prctl ran.
            if libc::getppid() == 1 {
                libc::_exit(0);
            }
            Ok(())
        });
    }

    command.spawn().ok()
}

/// Layer-shell settings for the launcher surface (centered, always-on-top).
#[cfg(target_os = "linux")]
fn launcher_layer_settings() -> iced_layershell::reexport::NewLayerShellSettings {
    use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer, NewLayerShellSettings};

    NewLayerShellSettings {
        size: Some(LAUNCHER_SIZE),
        anchor: Anchor::empty(),
        layer: Layer::Top,
        exclusive_zone: None,
        margin: None,
        keyboard_interactivity: KeyboardInteractivity::OnDemand,
        ..Default::default()
    }
}

/// Window settings for the launcher (non-Linux): a borderless, centered,
/// always-on-top window. Closing it does not exit the resident daemon.
#[cfg(not(target_os = "linux"))]
fn launcher_window_settings() -> iced::window::Settings {
    iced::window::Settings {
        size: iced::Size {
            width: LAUNCHER_SIZE.0 as f32,
            height: LAUNCHER_SIZE.1 as f32,
        },
        position: iced::window::Position::Centered,
        resizable: false,
        closeable: false,
        minimizable: false,
        decorations: false,
        transparent: true,
        blur: true,
        level: iced::window::Level::AlwaysOnTop,
        exit_on_close_request: false,
        ..Default::default()
    }
}

/// A subscription that listens on the IPC control socket and emits [`Message::Show`]
/// whenever a bare invocation asks the resident agent to open the launcher.
fn show_stream() -> impl iced::futures::Stream<Item = Message> {
    use iced::futures::channel::mpsc::Sender;
    iced::stream::channel(4, |output: Sender<Message>| async move {
        // The socket accept loop is blocking, so run it on a dedicated thread and
        // forward each request into the (async) subscription channel.
        let mut sender = output;
        std::thread::spawn(move || {
            if let Some(listener) = crate::ipc::bind() {
                crate::ipc::serve(listener, || {
                    let _ = sender.try_send(Message::Show);
                });
            }
        });
        // Keep the stream alive for the lifetime of the app.
        std::future::pending::<()>().await;
    })
}

#[cfg_attr(target_os = "linux", to_layer_message(multi))]
#[derive(Debug, Clone)]
pub enum Message {
    #[allow(dead_code)]
    IcedEvent(Event),
    PrismEvent(PrismEvent),
    Run,
    ExitApp,
    /// Show the launcher window/surface (IPC trigger / first launch).
    Show,
    /// Quit the resident agent entirely (e.g. from the tray).
    QuitAgent,
    /// A window (launcher or Plugin Manager) was closed.
    WindowClosed(iced::window::Id),
}
