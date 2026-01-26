mod items;
mod widgets;

use core::{get_entities, search::SearchEngine};
use std::collections::HashMap;

use iced::{
    Element, Length, Rectangle, Subscription, Task,
    advanced::widget::{operate, operation},
    event, keyboard,
    widget::{
        Id, column, container,
        operation::scroll_to,
        scrollable,
        selector::{self, Selector},
    },
};

use crate::{
    design_system::{colors, spacing},
    prism::items::ListEntry,
};

#[derive(Clone, Debug)]
pub struct PrismEntry {
    pub entry: ListEntry,
    pub id: Id,
}

impl From<ListEntry> for PrismEntry {
    fn from(entry: ListEntry) -> Self {
        Self {
            entry,
            id: Id::unique(),
        }
    }
}

pub struct Prism {
    query: String,
    all_entries: Vec<PrismEntry>,
    entries: Vec<PrismEntry>,
    selected_index: usize,
    search_id: Id,
    scroll_id: Id,
    viewport_height: f32,
    current_scroll_offset: f32,
    height_cache: HashMap<Id, f32>,
    default_row_height: f32,
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
    Scrolled(scrollable::Viewport),
    // Update the cache when a measurement finishes
    ItemMeasured { id: Id, rect: Rectangle },
}

impl Prism {
    pub fn new() -> (Self, Task<PrismEvent>) {
        let search_id = Id::unique();
        let scroll_id = Id::unique();

        let state = Prism {
            query: "".to_string(),
            all_entries: Vec::new(),
            entries: Vec::new(),
            selected_index: 0,
            search_id: search_id.clone(),
            scroll_id,
            viewport_height: 0.0,
            current_scroll_offset: 0.0,
            height_cache: HashMap::new(),
            default_row_height: 54.0, // A reasonable guess for start-up
        };

        let load_task = Task::perform(
            async { get_entities().into_iter().map(From::from).collect() },
            PrismEvent::EntriesLoaded,
        );
        let init_task = Task::perform(async {}, |_| PrismEvent::Initialized);

        (state, Task::batch(vec![load_task, init_task]))
    }

    pub fn update(&mut self, message: PrismEvent) -> Task<PrismEvent> {
        match message {
            PrismEvent::Initialized => iced::widget::operation::focus(self.search_id.clone()),

            PrismEvent::Scrolled(viewport) => {
                self.current_scroll_offset = viewport.absolute_offset().y;
                self.viewport_height = viewport.bounds().height;
                Task::none()
            }

            PrismEvent::EntriesLoaded(loaded_entries) => {
                let mut wrapped_entries: Vec<PrismEntry> =
                    loaded_entries.into_iter().map(PrismEntry::from).collect();

                wrapped_entries
                    .sort_by(|a, b| SearchEngine::compare(&a.entry.entity, &b.entry.entity));

                self.all_entries = wrapped_entries.clone();
                self.entries = wrapped_entries;

                // On load, measure the first item to calibrate our default height
                if let Some(first) = self.entries.first() {
                    return self.measure_item(first.id.clone());
                }
                Task::none()
            }

            PrismEvent::SearchInput(query) => {
                self.query = query;
                self.selected_index = 0;
                self.entries = self
                    .all_entries
                    .iter()
                    .filter(|e| SearchEngine::matches(&e.entry.entity, &self.query))
                    .cloned()
                    .collect();

                scroll_to(
                    self.scroll_id.clone(),
                    scrollable::AbsoluteOffset { x: 0.0, y: 0.0 },
                )
            }

            PrismEvent::SelectNext => {
                if !self.entries.is_empty() {
                    self.selected_index = (self.selected_index + 1).min(self.entries.len() - 1);
                    // 1. Scroll immediately based on cache (Fast)
                    let scroll_cmd = self.smart_scroll();
                    // 2. Measure the new item in background to keep cache accurate (Runtime)
                    let measure_cmd = self.measure_current_selection();

                    return Task::batch(vec![scroll_cmd, measure_cmd]);
                }
                Task::none()
            }

            PrismEvent::SelectPrevious => {
                self.selected_index = self.selected_index.saturating_sub(1);
                let scroll_cmd = self.smart_scroll();
                let measure_cmd = self.measure_current_selection();
                Task::batch(vec![scroll_cmd, measure_cmd])
            }

            PrismEvent::ItemMeasured { id, rect } => {
                // Update the cache with the real runtime height
                if rect.height > 0.0 {
                    self.height_cache.insert(id, rect.height);
                    // Update fallback height to match the most recently seen item
                    self.default_row_height = rect.height;
                }
                Task::none()
            }

            PrismEvent::EntrySelected(index) => {
                self.selected_index = index;
                if let Some(task) = self.execute_selected_entry(index) {
                    return task;
                }
                Task::none()
            }

            PrismEvent::Submit => {
                if let Some(task) = self.execute_selected_entry(self.selected_index) {
                    return task;
                }
                Task::none()
            }

            PrismEvent::Exit => iced::exit(),
        }
    }

    fn smart_scroll(&self) -> Task<PrismEvent> {
        let mut y_position = 0.0;
        let mut target_height = self.default_row_height;

        for i in 0..self.selected_index {
            if let Some(entry) = self.entries.get(i) {
                let h = *self
                    .height_cache
                    .get(&entry.id)
                    .unwrap_or(&self.default_row_height);
                y_position += h;
            }
        }

        if let Some(entry) = self.entries.get(self.selected_index) {
            target_height = *self
                .height_cache
                .get(&entry.id)
                .unwrap_or(&self.default_row_height);
        }

        let spacing = 0.0;
        y_position += spacing * (self.selected_index as f32);

        let item_top = y_position;
        let item_bottom = item_top + target_height;

        let view_top = self.current_scroll_offset;
        let view_bottom = view_top + self.viewport_height;

        if item_top < view_top {
            return scroll_to(
                self.scroll_id.clone(),
                scrollable::AbsoluteOffset {
                    x: 0.0,
                    y: item_top,
                },
            );
        } else if item_bottom > view_bottom && self.viewport_height > 0.0 {
            return scroll_to(
                self.scroll_id.clone(),
                scrollable::AbsoluteOffset {
                    x: 0.0,
                    y: item_bottom - self.viewport_height,
                },
            );
        }

        Task::none()
    }

    fn measure_current_selection(&self) -> Task<PrismEvent> {
        if let Some(entry) = self.entries.get(self.selected_index) {
            // Only measure if we don't have it cached yet,
            // OR if you suspect variable content might change height often
            if !self.height_cache.contains_key(&entry.id) {
                return self.measure_item(entry.id.clone());
            }
        }
        Task::none()
    }

    fn measure_item(&self, id: Id) -> Task<PrismEvent> {
        let selector = selector::id(id.clone()).find();
        let operation = operation::map(selector, move |v| PrismEvent::ItemMeasured {
            id: id.clone(),
            rect: v.unwrap().bounds(),
        });
        operate(operation)
    }

    fn execute_selected_entry(&mut self, index: usize) -> Option<Task<PrismEvent>> {
        if let Some(entry) = self.entries.get(index) {
            match entry.entry.execute() {
                Ok(_) => return Some(iced::exit()),
                Err(e) => eprintln!("Failed to launch: {}", e),
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
            container(widgets::list_item(
                &entry.entry,
                i == self.selected_index,
                PrismEvent::EntrySelected(i),
            ))
            .id(entry.id.clone())
            .into()
        });

        container(column![
            search_section,
            widgets::divider(),
            scrollable(column(list_section))
                .id(self.scroll_id.clone())
                .on_scroll(PrismEvent::Scrolled)
                .height(Length::Fill)
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
