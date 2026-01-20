use iced::{
    Alignment, Background, Color, Element, Length, Subscription, Task, event, gradient,
    keyboard::{self},
    widget::{
        button, column, container, image, row, scrollable, space::horizontal, text, text_input,
    },
};

use crate::design_system::{colors, icons, spacing, typo, typo::Typography};

#[derive(Default)]
pub struct Prism {
    query: String,
    entries: Vec<ListEntry>,
    selected_index: usize,
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
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
    SelectNext,
    SelectPrevious,
    EntrySelected(usize),
    Submit,
}

impl Prism {
    pub fn new_placeholder() -> Self {
        Prism {
            query: "".into(),
            selected_index: 0,
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
                // Reset selection when searching to avoid out-of-bounds
                self.selected_index = 0;
                Task::none()
            }
            PrismEvent::SelectNext => {
                if !self.entries.is_empty() {
                    self.selected_index = (self.selected_index + 1).min(self.entries.len() - 1);
                }
                Task::none()
            }
            PrismEvent::SelectPrevious => {
                self.selected_index = self.selected_index.saturating_sub(1);
                Task::none()
            }
            PrismEvent::EntrySelected(index) => {
                self.selected_index = index;
                println!("Selected: {}", self.entries[index].name);
                Task::none()
            }
            PrismEvent::Submit => {
                if let Some(entry) = self.entries.get(self.selected_index) {
                    println!("Launched via Enter: {}", entry.name);
                }
                Task::none()
            }
        }
    }

    pub fn subscription(&self) -> Subscription<PrismEvent> {
        event::listen_with(|event, _status, _window| {
            if let iced::Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) = event {
                match key {
                    keyboard::Key::Named(keyboard::key::Named::ArrowUp) => {
                        Some(PrismEvent::SelectPrevious)
                    }
                    keyboard::Key::Named(keyboard::key::Named::ArrowDown) => {
                        Some(PrismEvent::SelectNext)
                    }
                    keyboard::Key::Named(keyboard::key::Named::Enter) => Some(PrismEvent::Submit),
                    _ => None,
                }
            } else {
                None
            }
        })
    }

    pub fn view<'a>(&'a self) -> Element<'a, PrismEvent> {
        let input = text_input("Search for apps and commands...", &self.query)
            .on_input(PrismEvent::SearchInput)
            .size(typo::TITLE_L.0)
            .font(typo::TITLE_L.2)
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
            .enumerate()
            .map(|(i, ext)| {
                let kind: &str = ext.kind.into();
                let is_selected = i == self.selected_index;

                // WRAP IN BUTTON for easy Hover + Click handling
                button(
                    row![
                        image("assets/icon_placeholder.png")
                            .width(icons::LG)
                            .height(icons::LG),
                        column![
                            text(&ext.name)
                                .typography(typo::TITLE_M)
                                .color(colors::ON_SURFACE),
                            text(&ext.description)
                                .typography(typo::BODY_S)
                                .color(colors::ON_SURFACE_VARIANT),
                        ]
                        .spacing(spacing::SPACE_XXS),
                        horizontal(),
                        text(kind)
                            .typography(typo::LABEL_L)
                            .color(colors::ON_SURFACE_VARIANT),
                    ]
                    .spacing(spacing::SPACE_M)
                    .align_y(Alignment::Center),
                )
                .on_press(PrismEvent::EntrySelected(i))
                .width(Length::Fill)
                .padding(spacing::SPACE_S)
                .style(move |_theme, status| {
                    // 6. STYLING: Conditional background
                    let is_hovered = status == button::Status::Hovered;

                    let bg_color = if is_selected || is_hovered {
                        // Use a lighter/distinct color for selection/hover
                        colors::ON_SURFACE.scale_alpha(0.1)
                    } else {
                        Color::TRANSPARENT
                    };

                    button::Style {
                        background: Some(bg_color.into()),
                        text_color: colors::ON_SURFACE,
                        border: iced::Border {
                            radius: 8.0.into(),
                            ..iced::Border::default()
                        },
                        // rounded corners for list items
                        ..Default::default()
                    }
                })
                .into()
            })
            .collect();

        // Divider Line
        let gradient_line = container("")
            .width(Length::Fill)
            .height(1.0)
            .style(|_theme| {
                let fade_gradient = gradient::Linear::new(90.0)
                    .add_stop(0.0, Color::TRANSPARENT)
                    .add_stop(0.5, colors::ON_SURFACE)
                    .add_stop(1.0, Color::TRANSPARENT)
                    .into();

                container::Style {
                    background: Some(Background::Gradient(fade_gradient)),
                    ..container::Style::default()
                }
            });

        // Main Container
        container(column![
            input,
            gradient_line,
            scrollable(column(entries_list))
        ])
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(spacing::SPACE_S)
        .style(|_| container::Style {
            background: Some(colors::SURFACE_CONTAINER.scale_alpha(0.8).into()),
            border: iced::Border {
                color: colors::ON_SURFACE.scale_alpha(0.3),
                width: 1.0,
                radius: 15.0.into(),
            },
            ..Default::default()
        })
        .into()
    }
}
