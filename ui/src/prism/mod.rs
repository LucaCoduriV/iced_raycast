mod items;
mod keybindings;
pub mod state;
mod widgets;

use self::state::{PrismEntry, PrismState};
use crate::design_system::{colors, spacing};
use crate::prism::items::ListEntry;
use core::{AppState, get_entities, search::SearchEngine};
use iced::{
    Element, Length, Rectangle, Size, Subscription, Task,
    advanced::widget::{operate, operation},
    event, keyboard,
    widget::{
        Id, column, container,
        operation::{focus, scroll_to},
        scrollable,
        selector::{self, Selector},
    },
};

pub struct Prism {
    state: PrismState,
}

impl Prism {
    pub fn new() -> (Self, Task<PrismEvent>) {
        let search_id = Id::unique();
        let argument_id = Id::unique();
        let scroll_id = Id::unique();

        let state = PrismState {
            query: "".to_string(),
            argument: None,
            all_entries: Vec::new(),
            entries: Vec::new(),
            selected_index: 0,
            search_id: search_id.clone(),
            argument_id,
            scroll_id,
            viewport_height: 0.0,
            current_scroll_offset: 0.0,
            height_cache: std::collections::HashMap::new(),
            default_row_height: 54.0,
            show_argument_input: false,
            is_argument_input_active: false,
        };

        let load_task = Task::perform(
            async { get_entities().into_iter().map(From::from).collect() },
            PrismEvent::EntriesLoaded,
        );
        let init_task = Task::perform(async {}, |_| PrismEvent::Initialized);

        (Self { state }, Task::batch(vec![load_task, init_task]))
    }

    pub fn update(&mut self, message: PrismEvent, app_state: &mut AppState) -> Task<PrismEvent> {
        match message {
            PrismEvent::Initialized => focus(self.state.search_id.clone()),

            PrismEvent::Scrolled(viewport) => {
                self.state.current_scroll_offset = viewport.absolute_offset().y;
                self.state.viewport_height = viewport.bounds().height;
                Task::none()
            }

            PrismEvent::EntriesLoaded(loaded_entries) => {
                let mut wrapped_entries: Vec<PrismEntry> =
                    loaded_entries.into_iter().map(PrismEntry::from).collect();

                wrapped_entries.sort_by(|a, b| {
                    SearchEngine::compare(&a.entry.entity, &b.entry.entity, app_state)
                });

                self.state.all_entries = wrapped_entries.clone();
                self.state.entries = wrapped_entries;

                measure_all_visible_items(&self.state)
            }

            PrismEvent::SearchInput(query) => {
                self.state.query = query;
                self.state.selected_index = 0;
                self.state.argument = None;
                self.state.show_argument_input = false;
                self.state.is_argument_input_active = false;
                self.state.entries = self
                    .state
                    .all_entries
                    .iter()
                    .filter(|e| SearchEngine::matches(&e.entry.entity, &self.state.query))
                    .cloned()
                    .collect();

                Task::batch(vec![
                    scroll_to(
                        self.state.scroll_id.clone(),
                        scrollable::AbsoluteOffset { x: 0.0, y: 0.0 },
                    ),
                    measure_all_visible_items(&self.state),
                ])
            }

            PrismEvent::ArgumentInput(arg) => {
                self.state.argument = Some(arg);
                Task::none()
            }

            PrismEvent::SelectNext => {
                if !self.state.entries.is_empty() {
                    self.state.selected_index =
                        (self.state.selected_index + 1).min(self.state.entries.len() - 1);
                    return smart_scroll(&self.state);
                }
                Task::none()
            }

            PrismEvent::SelectPrevious => {
                self.state.selected_index = self.state.selected_index.saturating_sub(1);
                smart_scroll(&self.state)
            }

            PrismEvent::ItemMeasured { id, rect } => {
                if rect.height > 0.0 {
                    self.state.height_cache.insert(id, rect.height);
                    self.state.default_row_height = rect.height;
                }
                Task::none()
            }

            PrismEvent::EntrySelected(index) => {
                self.state.selected_index = index;
                if let Some(entry) = self.get_selected_entry() {
                    if entry.entry.entity.needs_argument() && self.get_argument().is_none() {
                        self.state.show_argument_input = true;
                        self.state.is_argument_input_active = true;
                        return focus(self.state.argument_id.clone());
                    }
                    self.state.is_argument_input_active = false;
                    return Task::batch(vec![
                        focus(self.state.search_id.clone()),
                        Task::done(PrismEvent::Run),
                    ]);
                }
                Task::none()
            }

            PrismEvent::Submit => {
                // If there's a selected entry, treat Submit as if that entry was selected by a click.
                // This ensures the selected_index is properly handled and the command runs if applicable.
                if !self.state.entries.is_empty() {
                    return self.update(
                        PrismEvent::EntrySelected(self.state.selected_index),
                        app_state,
                    );
                }
                Task::none()
            }

            PrismEvent::EscapePressed => {
                if self.state.is_argument_input_active {
                    self.state.argument = Option::None;
                    self.state.show_argument_input = false;
                    self.state.is_argument_input_active = false;
                    focus(self.state.search_id.clone())
                } else {
                    Task::done(PrismEvent::ExitApp)
                }
            }

            _ => Task::none(),
        }
    }

    pub fn get_argument(&self) -> Option<String> {
        self.state.argument.clone()
    }

    pub fn get_selected_entry(&self) -> Option<&PrismEntry> {
        self.state.entries.get(self.state.selected_index)
    }

    pub fn subscription(&self) -> Subscription<PrismEvent> {
        event::listen_with(|event, _status, _window| {
            if let iced::Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) = event {
                keybindings::map_key_to_action(key).map(|action| match action {
                    keybindings::KeyAction::SelectPrevious => PrismEvent::SelectPrevious,
                    keybindings::KeyAction::SelectNext => PrismEvent::SelectNext,
                    keybindings::KeyAction::Submit => PrismEvent::Submit,
                    keybindings::KeyAction::EscapePressed => PrismEvent::EscapePressed,
                })
            } else {
                None
            }
        })
    }

    pub fn view<'a>(&'a self) -> Element<'a, PrismEvent> {
        let selected_entry = self.get_selected_entry();
        let search_section = widgets::search_bar(
            self.state.search_id.clone(),
            &self.state.query,
            PrismEvent::SearchInput,
            self.state.argument_id.clone(),
            self.state.argument.as_deref(),
            PrismEvent::ArgumentInput,
            selected_entry.and_then(|e| e.entry.entity.icon()),
            self.state.show_argument_input,
        );

        let list_section = self.state.entries.iter().enumerate().map(|(i, entry)| {
            container(widgets::list_item(
                &entry.entry,
                i == self.state.selected_index,
                PrismEvent::EntrySelected(i),
            ))
            .id(entry.id.clone())
            .into()
        });

        container(column![
            search_section,
            widgets::divider(),
            scrollable(column(list_section))
                .id(self.state.scroll_id.clone())
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

#[derive(Debug, Clone)]
pub enum PrismEvent {
    Initialized,
    SearchInput(String),
    ArgumentInput(String),
    SelectNext,
    SelectPrevious,
    EntrySelected(usize),
    Submit,
    EntriesLoaded(Vec<ListEntry>),

    Scrolled(scrollable::Viewport),
    ItemMeasured { id: Id, rect: Rectangle },
    Run,
    EscapePressed,
    ExitApp, // New variant for exiting the application
}

fn measure_all_visible_items(state: &PrismState) -> Task<PrismEvent> {
    let tasks: Vec<Task<PrismEvent>> = state
        .entries
        .iter()
        .map(|entry| measure_item(entry.id.clone()))
        .collect();

    Task::batch(tasks)
}

fn measure_item(id: Id) -> Task<PrismEvent> {
    let selector = selector::id(id.clone()).find();
    let operation = operation::map(selector, move |v| {
        v.map(|widget| PrismEvent::ItemMeasured {
            id: id.clone(),
            rect: widget.bounds(),
        })
        .unwrap_or(PrismEvent::ItemMeasured {
            id: id.clone(),
            rect: Rectangle::with_size(Size::new(0.0, 0.0)),
        })
    });
    operate(operation)
}

fn smart_scroll(state: &PrismState) -> Task<PrismEvent> {
    let mut y_position = 0.0;
    let mut target_height = state.default_row_height;

    for i in 0..state.selected_index {
        if let Some(entry) = state.entries.get(i) {
            let h = *state
                .height_cache
                .get(&entry.id)
                .unwrap_or(&state.default_row_height);
            y_position += h;
        }
    }

    if let Some(entry) = state.entries.get(state.selected_index) {
        target_height = *state
            .height_cache
            .get(&entry.id)
            .unwrap_or(&state.default_row_height);
    }

    let item_top = y_position;
    let item_bottom = item_top + target_height;

    let view_top = state.current_scroll_offset;
    let view_bottom = view_top + state.viewport_height;

    if item_top < view_top {
        return scroll_to(
            state.scroll_id.clone(),
            scrollable::AbsoluteOffset {
                x: 0.0,
                y: item_top,
            },
        );
    } else if item_bottom > view_bottom && state.viewport_height > 0.0 {
        return scroll_to(
            state.scroll_id.clone(),
            scrollable::AbsoluteOffset {
                x: 0.0,
                y: item_bottom - state.viewport_height,
            },
        );
    }

    Task::none()
}
