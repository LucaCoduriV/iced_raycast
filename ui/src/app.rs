use std::process;

use core::AppState;

use iced::{Color, Element, Event, Task, event, widget::container};
#[cfg(target_os = "linux")]
use iced_layershell::to_layer_message;

use crate::prism;
use crate::prism::PrismEvent;

pub struct Raycast {
    prism: prism::Prism,
    app_state: AppState,
}

impl Raycast {
    pub fn new() -> (Raycast, Task<Message>) {
        let app_state = AppState::load();
        let (prism, prism_task) = prism::Prism::new();

        let state = Raycast { prism, app_state };

        (state, prism_task.map(Message::PrismEvent))
    }

    pub fn namespace() -> String {
        String::from("RaycastClone")
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::PrismEvent(prism_event) => {
                let task = self.prism.update(prism_event, &mut self.app_state);
                task.map(|event| {
                    if matches!(event, PrismEvent::Run) {
                        Message::Run
                    } else if matches!(event, PrismEvent::ExitApp) {
                        Message::ExitApp // Explicitly map PrismEvent::ExitApp to top-level Message::ExitApp
                    } else {
                        Message::PrismEvent(event)
                    }
                })
            }
            Message::Run => {
                if let Some(entry) = self.prism.get_selected_entry().cloned() {
                    self.app_state.record_usage(&entry.entry.entity);
                    if let Err(e) = self.app_state.save() {
                        eprintln!("Failed to save state: {}", e);
                    }

                    let argument = self.prism.get_argument();
                    if let Err(e) = entry.entry.execute(Some(argument)) {
                        eprintln!("Failed to launch: {}", e);
                    }
                }
                Task::perform(
                    async {
                        process::exit(0);
                    },
                    |_| Message::IcedEvent(Event::Window(iced::window::Event::Closed)),
                )
            }
            Message::ExitApp => Task::perform(
                async {
                    process::exit(0);
                },
                |_| Message::IcedEvent(Event::Window(iced::window::Event::Closed)),
            ),
            _ => Task::none(),
        }
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        use iced::Subscription;

        Subscription::batch(vec![
            event::listen().map(Message::IcedEvent),
            self.prism.subscription().map(|event| match event {
                PrismEvent::ExitApp => Message::ExitApp,
                _ => Message::PrismEvent(event),
            }),
        ])
    }

    pub fn view<'a>(&'a self) -> Element<'a, Message> {
        container(self.prism.view().map(Message::PrismEvent)).into()
    }

    pub fn style(&self, _theme: &iced::Theme) -> iced::theme::Style {
        iced::theme::Style {
            background_color: Color::TRANSPARENT,
            text_color: Color::WHITE,
        }
    }
}

#[cfg_attr(target_os = "linux", to_layer_message)]
#[derive(Debug, Clone)]
pub enum Message {
    #[allow(dead_code)]
    IcedEvent(Event),
    PrismEvent(PrismEvent),
    Run,
    ExitApp, // New variant to signal application exit
}
