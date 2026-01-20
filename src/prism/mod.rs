mod component;

use iced::{
    Element, Length, Subscription, Task, event, keyboard,
    widget::{column, container, scrollable},
};

use crate::design_system::{colors, spacing};

#[derive(Default)]
pub struct Prism {
    query: String,
    entries: Vec<ListEntry>,
    selected_index: usize,
}

#[derive(Debug, Clone)]
pub enum PrismEvent {
    SearchInput(String),
    SelectNext,
    SelectPrevious,
    EntrySelected(usize),
    Submit,
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

pub struct ListEntry {
    name: String,
    description: String,
    kind: ListEntryKind,
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
        let search_section = component::search_bar(&self.query, PrismEvent::SearchInput);

        let list_section = self.entries.iter().enumerate().map(|(i, entry)| {
            component::list_item(
                entry,
                i == self.selected_index,
                PrismEvent::EntrySelected(i),
            )
        });

        container(column![
            search_section,
            component::divider(),
            scrollable(column(list_section))
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
