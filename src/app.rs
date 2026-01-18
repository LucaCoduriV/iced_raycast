use iced::{
    Color, Element, Event, Task, event,
    widget::{container, text},
};
use iced_layershell::to_layer_message;

use crate::prism::{Prism, PrismEvent};

pub struct Raycast {
    prism: Prism,
}

impl Default for Raycast {
    fn default() -> Self {
        Self {
            prism: Prism::default(),
        }
    }
}

impl Raycast {
    pub fn namespace() -> String {
        String::from("RaycastClone")
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            _ => Task::none(),
        }
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        event::listen().map(Message::IcedEvent)
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

#[to_layer_message]
#[derive(Debug, Clone)]
pub enum Message {
    SearchInput(String),
    IcedEvent(Event),
    PrismEvent(PrismEvent),
    Close,
}
