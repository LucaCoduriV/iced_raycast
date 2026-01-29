use core::AppState;

use iced::{Color, Element, Event, Task, event, widget::container};
#[cfg(target_os = "linux")]
use iced_layershell::to_layer_message;

use crate::prism;
use crate::prism::PrismEvent;

pub struct Raycast {
    prism: prism::state::PrismState,
    app_state: AppState,
}

impl Raycast {
    pub fn new() -> (Raycast, Task<Message>) {
        let app_state = AppState::load();
        let (prism, prism_task) = prism::new(app_state.clone());

        let state = Raycast { prism, app_state };

        (state, prism_task.map(Message::PrismEvent))
    }

    pub fn namespace() -> String {
        String::from("RaycastClone")
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::PrismEvent(prism_event) => {
                if let PrismEvent::Submit = prism_event {
                    if let Some(entry) = prism::get_selected_entry(&self.prism).cloned() {
                        self.app_state.record_usage(&entry.entry.entity);
                        if let Err(e) = self.app_state.save() {
                            eprintln!("Failed to save state: {}", e);
                        }

                        if let Err(e) = entry.entry.execute() {
                            eprintln!("Failed to launch: {}", e);
                        }

                        let new_state = self.app_state.clone();
                        return Task::perform(async {}, move |_| {
                            Message::PrismEvent(PrismEvent::StateUpdated(new_state))
                        });
                    }
                }
                prism::update(&mut self.prism, prism_event).map(Message::PrismEvent)
            }
            _ => Task::none(),
        }
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        use iced::Subscription;

        Subscription::batch(vec![
            event::listen().map(Message::IcedEvent),
            prism::subscription().map(Message::PrismEvent),
        ])
    }

    pub fn view<'a>(&'a self) -> Element<'a, Message> {
        container(prism::view(&self.prism).map(Message::PrismEvent)).into()
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
}
