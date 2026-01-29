use core::AppState;

use iced::{Color, Element, Event, Task, event, widget::container};
#[cfg(target_os = "linux")]
use iced_layershell::to_layer_message;

use crate::prism::{Prism, PrismEvent};

pub struct Raycast {
    prism: Prism,
    app_state: AppState,
}

impl Raycast {
    pub fn new() -> (Raycast, Task<Message>) {
        let (prism, prism_task) = Prism::new();

        let state = Raycast {
            prism,
            app_state: AppState::load(),
        };

        (state, prism_task.map(Message::PrismEvent))
    }

    pub fn namespace() -> String {
        String::from("RaycastClone")
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::PrismEvent(prism_event) => {
                self.prism.update(prism_event).map(Message::PrismEvent)
            }
            _ => Task::none(),
        }
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        use iced::Subscription;

        Subscription::batch(vec![
            event::listen().map(Message::IcedEvent),
            self.prism.subscription().map(Message::PrismEvent),
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
}
