use iced::{
    Alignment, Background, Color, Element, Length, Task, gradient,
    widget::{column, container, row, scrollable, space::horizontal, text, text_input},
};

#[derive(Default)]
pub struct Prism {
    query: String,
    entries: Vec<ListEntry>,
}

#[derive(Clone, Copy)]
enum ListEntryKind {
    Command,
    Application,
}

impl From<ListEntryKind> for &str {
    fn from(val: ListEntryKind) -> Self {
        match val {
            ListEntryKind::Command => "Command",
            ListEntryKind::Application => "Application",
        }
    }
}

struct ListEntry {
    name: String,
    description: String,
    kind: ListEntryKind,
}

#[derive(Debug, Clone)]
pub enum PrismEvent {
    SearchInput(String),
}

impl Prism {
    pub fn new_placeholder() -> Self {
        Prism {
            query: "".into(),
            entries: vec![
                ListEntry {
                    name: "Firefox".into(),
                    description: "Browser".into(),
                    kind: ListEntryKind::Application,
                },
                ListEntry {
                    name: "Chrome".into(),
                    description: "Browser".into(),
                    kind: ListEntryKind::Application,
                },
                ListEntry {
                    name: "Vivaldi".into(),
                    description: "Browser".into(),
                    kind: ListEntryKind::Application,
                },
                ListEntry {
                    name: "Zen Browser".into(),
                    description: "Browser".into(),
                    kind: ListEntryKind::Application,
                },
            ],
        }
    }

    pub fn update(&mut self, message: PrismEvent) -> Task<PrismEvent> {
        match message {
            PrismEvent::SearchInput(query) => {
                self.query = query;
                Task::none()
            }
        }
    }

    pub fn view<'a>(&'a self) -> Element<'a, PrismEvent> {
        let input = text_input("Search for apps and commands...", &self.query)
            .on_input(PrismEvent::SearchInput)
            .size(20)
            .padding(15)
            .style(|_theme, _status| text_input::Style {
                background: Color::TRANSPARENT.into(),
                border: iced::Border {
                    width: 0.0,
                    ..Default::default()
                },
                icon: Color::WHITE,
                placeholder: Color::WHITE,
                value: Color::WHITE,
                selection: Color::WHITE,
            });

        let entries_list: Vec<Element<'a, PrismEvent>> = self
            .entries
            .iter()
            .map(|ext| {
                let kind: &str = ext.kind.into();
                container(
                    row![
                        container(text(""))
                            .width(32)
                            .height(32)
                            .style(|_| container::Style {
                                background: Some(Color::from_rgb(0.2, 0.2, 0.2).into()),
                                border: iced::Border {
                                    radius: 8.0.into(),
                                    ..Default::default()
                                },
                                ..Default::default()
                            }),
                        column![
                            text(&ext.name)
                                .size(16)
                                .color(Color::from_rgb(0.7, 0.7, 0.7)),
                            text(&ext.description)
                                .size(12)
                                .color(Color::from_rgb(0.4, 0.4, 0.4)),
                        ]
                        .spacing(2),
                        horizontal(),
                        text(kind).size(16).color(Color::from_rgb(0.7, 0.7, 0.7)),
                    ]
                    .spacing(15)
                    .align_y(Alignment::Center),
                )
                .padding(10)
                .width(Length::Fill)
                .style(move |_| container::Style::default())
                .into()
            })
            .collect();

        let gradient_line = container("")
            .width(Length::Fill) // The line stretches horizontally
            .height(1.0) // The thickness of your line
            .style(|_theme| {
                let fade_gradient = gradient::Linear::new(90.0) // Left to right
                    .add_stop(0.0, Color::TRANSPARENT) // Start transparent
                    .add_stop(0.5, Color::WHITE) // Peak at white in the middle
                    .add_stop(1.0, Color::TRANSPARENT) // End transparent
                    .into();

                container::Style {
                    background: Some(Background::Gradient(fade_gradient)),
                    ..container::Style::default()
                }
            });

        container(column![
            input,
            gradient_line,
            scrollable(column(entries_list))
        ])
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(10)
        .style(|_| container::Style {
            background: Some(Color::from_rgba(0.12, 0.12, 0.12, 0.7).into()),
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
