mod items;
mod widgets;

use core::{get_entities, search::SearchEngine};

use iced::{
    Element, Length, Subscription, Task, event, keyboard,
    widget::{Id, column, container, operation::focus, scrollable},
};

use crate::{
    design_system::{colors, spacing},
    prism::items::ListEntry,
};

pub struct Prism {
    query: String,
    all_entries: Vec<ListEntry>,
    entries: Vec<ListEntry>,
    selected_index: usize,
    search_id: Id,
}

#[derive(Debug, Clone)]
pub enum PrismEvent {
    Initialized,
    SearchInput(String),
    SelectNext,
    SelectPrevious,
    EntrySelected(usize),
    Submit,
    EntriesLoaded(Vec<ListEntry>),
    Exit,
}

impl Prism {
    pub fn new() -> (Self, Task<PrismEvent>) {
        let search_id = iced::widget::Id::unique();

        let state = Prism {
            query: "".to_string(),
            all_entries: Vec::new(),
            entries: Vec::new(),
            selected_index: 0,
            search_id: search_id.clone(),
        };

        let load_task = Task::perform(
            async { get_entities().into_iter().map(From::from).collect() },
            PrismEvent::EntriesLoaded,
        );
        let init_task = Task::perform(async {}, |_| PrismEvent::Initialized);

        let task = Task::batch(vec![load_task, init_task]);

        (state, task)
    }

    pub fn update(&mut self, message: PrismEvent) -> Task<PrismEvent> {
        match message {
            PrismEvent::Initialized => focus(self.search_id.clone()),
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
                if let Some(value) = self.execute_selected_entry(index) {
                    return value;
                }
                Task::none()
            }
            PrismEvent::Submit => {
                if let Some(value) = self.execute_selected_entry(self.selected_index) {
                    return value;
                }
                Task::none()
            }
            PrismEvent::Exit => iced::exit(),
        }
    }

    fn execute_selected_entry(&mut self, index: usize) -> Option<Task<PrismEvent>> {
        if let Some(entry) = self.entries.get(index) {
            match entry.execute() {
                Ok(_) => {
                    println!("Launched: {}", entry.name());
                    return Some(iced::exit());
                }
                Err(e) => {
                    eprintln!("Failed to launch: {}", e);
                }
            }
        }
        None
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
                    keyboard::Key::Named(keyboard::key::Named::Escape) => Some(PrismEvent::Exit),
                    _ => None,
                }
            } else {
                None
            }
        })
    }

    pub fn view<'a>(&'a self) -> Element<'a, PrismEvent> {
        let search_section =
            widgets::search_bar(self.search_id.clone(), &self.query, PrismEvent::SearchInput);

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
