use iced::{
    Color, Element, Length, Theme,
    theme::Palette,
    widget::{Container, button, container, row, text},
};

#[derive(Default)]
pub struct Prism {
    query: String,
    extensions: Vec<ListEntry>,
}

enum ListEntry {
    Command,
    Program { name: String, description: String },
}

#[derive(Debug, Clone)]
pub enum PrismEvent {
    Test,
}

impl Prism {
    pub fn view<'a>(&'a self) -> Element<'a, PrismEvent> {
        container(row![
            button("-").on_press(PrismEvent::Test),
            text("Test"),
            button("+").on_press(PrismEvent::Test),
        ])
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(10)
        .style(|_| container::Style {
            background: Some(Color::from_rgb(0.12, 0.12, 0.12).into()),
            border: iced::Border {
                color: Color::from_rgba(1.0, 1.0, 1.0, 0.1),
                width: 1.0,
                radius: 15.0.into(),
            },
            ..Default::default()
        })
        .into()
    }
}
