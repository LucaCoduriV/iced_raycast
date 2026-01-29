mod items;
pub mod state;
mod widgets;

use crate::prism::items::ListEntry;
use self::state::{PrismEntry, PrismState};
use crate::design_system::{colors, spacing};
use core::{get_entities, search::SearchEngine, AppState};
use iced::{
    advanced::widget::{operate, operation},
    event, keyboard,
    widget::{
        column, container,
        operation::scroll_to,
        scrollable,
        selector::{self, Selector},
        Id,
    },
    Element, Length, Rectangle, Size, Subscription, Task,
};

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
    ItemMeasured { id: Id, rect: Rectangle },
    StateUpdated(AppState),
}

pub fn new(app_state: AppState) -> (PrismState, Task<PrismEvent>) {
    let search_id = Id::unique();
    let scroll_id = Id::unique();

    let state = PrismState {
        query: "".to_string(),
        all_entries: Vec::new(),
        entries: Vec::new(),
        selected_index: 0,
        search_id: search_id.clone(),
        scroll_id,
        viewport_height: 0.0,
        current_scroll_offset: 0.0,
        height_cache: std::collections::HashMap::new(),
        default_row_height: 54.0,
        app_state,
    };

    let load_task = Task::perform(
        async { get_entities().into_iter().map(From::from).collect() },
        PrismEvent::EntriesLoaded,
    );
    let init_task = Task::perform(async {}, |_| PrismEvent::Initialized);

    (state, Task::batch(vec![load_task, init_task]))
}

pub fn update(state: &mut PrismState, message: PrismEvent) -> Task<PrismEvent> {
    match message {
        PrismEvent::Initialized => iced::widget::operation::focus(state.search_id.clone()),

        PrismEvent::Scrolled(viewport) => {
            state.current_scroll_offset = viewport.absolute_offset().y;
            state.viewport_height = viewport.bounds().height;
            Task::none()
        }

        PrismEvent::EntriesLoaded(loaded_entries) => {
            let mut wrapped_entries: Vec<PrismEntry> =
                loaded_entries.into_iter().map(PrismEntry::from).collect();

            wrapped_entries.sort_by(|a, b| {
                SearchEngine::compare(&a.entry.entity, &b.entry.entity, &state.app_state)
            });

            state.all_entries = wrapped_entries.clone();
            state.entries = wrapped_entries;

            // Measure every item in the list immediately upon loading
            measure_all_visible_items(state)
        }

        PrismEvent::SearchInput(query) => {
            state.query = query;
            state.selected_index = 0;
            state.entries = state
                .all_entries
                .iter()
                .filter(|e| SearchEngine::matches(&e.entry.entity, &state.query))
                .cloned()
                .collect();

            Task::batch(vec![
                scroll_to(
                    state.scroll_id.clone(),
                    scrollable::AbsoluteOffset { x: 0.0, y: 0.0 },
                ),
                measure_all_visible_items(state),
            ])
        }

        PrismEvent::SelectNext => {
            if !state.entries.is_empty() {
                state.selected_index = (state.selected_index + 1).min(state.entries.len() - 1);
                return smart_scroll(state);
            }
            Task::none()
        }

        PrismEvent::SelectPrevious => {
            state.selected_index = state.selected_index.saturating_sub(1);
            smart_scroll(state)
        }

        PrismEvent::ItemMeasured { id, rect } => {
            if rect.height > 0.0 {
                state.height_cache.insert(id, rect.height);
                state.default_row_height = rect.height;
            }
            Task::none()
        }

        PrismEvent::EntrySelected(index) => {
            state.selected_index = index;
            Task::none()
        }

        PrismEvent::Submit => {
            // We let the parent handle the execution
            // We just need to wait for it to be done
            Task::none()
        }

        PrismEvent::StateUpdated(new_state) => {
            state.app_state = new_state;
            state.all_entries.sort_by(|a, b| {
                SearchEngine::compare(&a.entry.entity, &b.entry.entity, &state.app_state)
            });
            state.entries.sort_by(|a, b| {
                SearchEngine::compare(&a.entry.entity, &b.entry.entity, &state.app_state)
            });
            Task::none()
        }

        PrismEvent::Exit => iced::exit(),
    }
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

pub fn get_selected_entry(state: &PrismState) -> Option<&PrismEntry> {
    state.entries.get(state.selected_index)
}

pub fn subscription() -> Subscription<PrismEvent> {
    event::listen_with(|event, _status, _window| {
        if let iced::Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) = event {
            match key {
                keyboard::Key::Named(keyboard::key::Named::ArrowUp) => Some(PrismEvent::SelectPrevious),
                keyboard::Key::Named(keyboard::key::Named::ArrowDown) => Some(PrismEvent::SelectNext),
                keyboard::Key::Named(keyboard::key::Named::Enter) => Some(PrismEvent::Submit),
                keyboard::Key::Named(keyboard::key::Named::Escape) => Some(PrismEvent::Exit),
                _ => None,
            }
        } else {
            None
        }
    })
}

pub fn view<'a>(state: &'a PrismState) -> Element<'a, PrismEvent> {
    let search_section = widgets::search_bar(
        state.search_id.clone(),
        &state.query,
        PrismEvent::SearchInput,
    );

    let list_section = state.entries.iter().enumerate().map(|(i, entry)| {
        container(widgets::list_item(
            &entry.entry,
            i == state.selected_index,
            PrismEvent::EntrySelected(i),
        ))
        .id(entry.id.clone())
        .into()
    });

    container(column![
        search_section,
        widgets::divider(),
        scrollable(column(list_section))
            .id(state.scroll_id.clone())
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
