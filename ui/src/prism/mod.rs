mod items;
mod widgets;

use core::{get_entities, search::SearchEngine};

use iced::{
    Element, Length, Subscription, Task, event, keyboard,
    widget::{column, container, scrollable},
};

use crate::{
    design_system::{colors, spacing},
    prism::items::ListEntry,
};

#[derive(Default)]
pub struct Prism {
    query: String,
    all_entries: Vec<ListEntry>,
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
    EntriesLoaded(Vec<ListEntry>),
}

impl Prism {
    pub fn new() -> (Self, Task<PrismEvent>) {
        let state = Prism {
            query: "".to_string(),
            all_entries: Vec::new(),
            entries: Vec::new(),
            selected_index: 0,
        };
        let load_task = Task::perform(
            async { get_entities().into_iter().map(From::from).collect() },
            PrismEvent::EntriesLoaded,
        );

        (state, load_task)
    }

    pub fn update(&mut self, message: PrismEvent) -> Task<PrismEvent> {
        match message {
            PrismEvent::EntriesLoaded(mut loaded_entries) => {
                loaded_entries.sort_by(|a, b| SearchEngine::compare(&a.entity, &b.entity));

                self.all_entries = loaded_entries.clone();
                self.entries = loaded_entries;
                Task::none()
            }
            PrismEvent::SearchInput(query) => {
                self.query = query;
                self.selected_index = 0;

                self.entries = self
                    .all_entries
                    .iter()
                    .filter(|list_entry| SearchEngine::matches(&list_entry.entity, &self.query))
                    .cloned()
                    .collect();

                Task::none()
            }
            PrismEvent::SelectNext => {
                if !self.all_entries.is_empty() {
                    self.selected_index = (self.selected_index + 1).min(self.all_entries.len() - 1);
                }
                Task::none()
            }
            PrismEvent::SelectPrevious => {
                self.selected_index = self.selected_index.saturating_sub(1);
                Task::none()
            }
            PrismEvent::EntrySelected(index) => {
                self.selected_index = index;
                if let Some(entry) = self.all_entries.get(index) {
                    entry.execute();
                    println!("Selected: {}", entry.name());
                }
                Task::none()
            }
            PrismEvent::Submit => {
                if let Some(entry) = self.all_entries.get(self.selected_index) {
                    match entry.entity.execute() {
                        Ok(_) => {
                            println!("Launched: {}", entry.entity.name());
                            return iced::window::latest().and_then(iced::window::close);
                        }
                        Err(e) => {
                            eprintln!("Failed to launch: {}", e);
                        }
                    }
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
        let search_section = widgets::search_bar(&self.query, PrismEvent::SearchInput);

        let list_section = self.entries.iter().enumerate().map(|(i, entry)| {
            widgets::list_item(
                entry,
                i == self.selected_index,
                PrismEvent::EntrySelected(i),
            )
        });

        container(column![
            search_section,
            widgets::divider(),
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
