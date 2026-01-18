use iced::{
    Alignment, Color, Element, Length, Task,
    widget::{column, container, row, scrollable, text, text_input},
};

#[derive(Default)]
pub struct Prism {
    query: String,
    entries: Vec<ListEntry>,
}

enum ListEntryKind {
    Command,
    Program,
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
                    kind: ListEntryKind::Program,
                },
                ListEntry {
                    name: "Chrome".into(),
                    description: "Browser".into(),
                    kind: ListEntryKind::Program,
                },
                ListEntry {
                    name: "Vivaldi".into(),
                    description: "Browser".into(),
                    kind: ListEntryKind::Program,
                },
                ListEntry {
                    name: "Zen Browser".into(),
                    description: "Browser".into(),
                    kind: ListEntryKind::Program,
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
                        .spacing(2)
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

        container(column![input, scrollable(column(entries_list))])
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
