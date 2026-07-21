mod items;
mod keybindings;
pub mod state;
mod views;
mod widgets;

use std::sync::Arc;

use self::state::{PrismEntry, PrismState, ViewState};
use crate::design_system::{colors, spacing};
use crate::prism::items::ListEntry;
use core::{
    ActionEffect, AppState, Entity, FieldValueKind, PluginRegistry, ViewEvent, ViewEventKind,
    ViewResponse, get_entities, search::SearchEngine,
};
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
    registry: Arc<PluginRegistry>,
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
            show_actions: false,
            actions_selected_index: 0,
            recent_arguments: Vec::new(),
            views: Vec::new(),
        };

        let load_task = Task::perform(
            async { get_entities().into_iter().map(From::from).collect() },
            PrismEvent::EntriesLoaded,
        );
        let init_task = Task::perform(async {}, |_| PrismEvent::Initialized);

        (
            Self {
                state,
                registry: Arc::new(PluginRegistry::with_builtins()),
            },
            Task::batch(vec![load_task, init_task]),
        )
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

                // Group by section (applications ahead of commands), then rank
                // within each group by usage score / name.
                wrapped_entries.sort_by(|a, b| {
                    a.entry
                        .section_rank()
                        .cmp(&b.entry.section_rank())
                        .then_with(|| {
                            SearchEngine::compare(&a.entry.entity, &b.entry.entity, app_state)
                        })
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
                self.state.show_actions = false;
                self.state.actions_selected_index = 0;
                // Query-driven plugin results (e.g. calculator) are produced
                // fresh for this query and shown ahead of the filtered list;
                // they bypass the name/description haystack filter.
                let mut entries: Vec<PrismEntry> = self
                    .registry
                    .query(&self.state.query)
                    .into_iter()
                    .map(|result| PrismEntry::from(ListEntry::from(Entity::Plugin(result))))
                    .collect();

                let query_lower = self.state.query.to_lowercase();
                entries.extend(
                    self.state
                        .all_entries
                        .iter()
                        .filter(|e| SearchEngine::matches(&e.search_haystack, &query_lower))
                        .cloned(),
                );
                self.state.entries = entries;

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

            PrismEvent::ArgumentSelected(value) => {
                // Picking a recent argument fills it in and runs immediately.
                self.state.argument = Some(value);
                Task::done(PrismEvent::Run)
            }

            PrismEvent::SelectNext => {
                if let Some(top) = self.state.views.last_mut() {
                    // Down moves one grid row.
                    let (len, columns) = (top.grid_len(), top.grid_columns());
                    if len > 0 {
                        top.selected = (top.selected + columns).min(len - 1);
                    }
                    return Task::none();
                }
                if self.state.show_actions {
                    let len = self.actions_len();
                    if len > 0 {
                        self.state.actions_selected_index =
                            (self.state.actions_selected_index + 1).min(len - 1);
                    }
                    return Task::none();
                }
                if self.state.show_argument_input {
                    return Task::none();
                }
                if !self.state.entries.is_empty() {
                    self.state.selected_index =
                        (self.state.selected_index + 1).min(self.state.entries.len() - 1);
                    return smart_scroll(&self.state);
                }
                Task::none()
            }

            PrismEvent::SelectPrevious => {
                if let Some(top) = self.state.views.last_mut() {
                    // Up moves one grid row.
                    top.selected = top.selected.saturating_sub(top.grid_columns());
                    return Task::none();
                }
                if self.state.show_actions {
                    self.state.actions_selected_index =
                        self.state.actions_selected_index.saturating_sub(1);
                    return Task::none();
                }
                if self.state.show_argument_input {
                    return Task::none();
                }
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

                // Plugin results carry an effect (copy / push a view / close);
                // handle it here rather than the app-launch path.
                let effect = self.get_selected_entry().and_then(|entry| {
                    let entity = &entry.entry.entity;
                    entity.primary_effect().map(|effect| {
                        (
                            effect,
                            entity.plugin_source_id().unwrap_or_default().to_string(),
                        )
                    })
                });
                if let Some((effect, plugin_id)) = effect {
                    return self.apply_effect(effect, plugin_id);
                }

                // Read what we need before mutating self below.
                let selection = self
                    .get_selected_entry()
                    .map(|entry| (entry.entry.entity.needs_argument(), entry.entry.name().to_string()));

                let Some((needs_argument, name)) = selection else {
                    return Task::none();
                };

                if needs_argument && self.get_argument().is_none() {
                    self.state.show_argument_input = true;
                    self.state.is_argument_input_active = true;
                    self.state.recent_arguments = app_state.recent_arguments(&name);
                    return focus(self.state.argument_id.clone());
                }

                self.state.is_argument_input_active = false;
                Task::batch(vec![
                    focus(self.state.search_id.clone()),
                    Task::done(PrismEvent::Run),
                ])
            }

            PrismEvent::Submit => {
                if !self.state.views.is_empty() {
                    return self.update(PrismEvent::ViewSubmit, app_state);
                }
                if self.state.show_actions {
                    return self.update(
                        PrismEvent::InvokeAction(self.state.actions_selected_index),
                        app_state,
                    );
                }
                if !self.state.entries.is_empty() {
                    return self.update(
                        PrismEvent::EntrySelected(self.state.selected_index),
                        app_state,
                    );
                }
                Task::none()
            }

            PrismEvent::ToggleActions => {
                if self.get_selected_entry().is_some() {
                    self.state.show_actions = !self.state.show_actions;
                    self.state.actions_selected_index = 0;
                }
                Task::none()
            }

            PrismEvent::InvokeAction(index) => {
                self.state.actions_selected_index = index;

                // Resolve the action's data before mutating self, so we don't
                // hold a borrow across the follow-up update/task.
                let resolved = self.get_selected_entry().and_then(|entry| {
                    let plugin_id = entry.entry.entity.plugin_source_id().unwrap_or_default().to_string();
                    widgets::actions_for(&entry.entry)
                        .into_iter()
                        .nth(index)
                        .map(|action| (action.kind, entry.entry.name().to_string(), plugin_id))
                });

                self.state.show_actions = false;

                match resolved {
                    Some((widgets::MenuActionKind::Primary, _, _)) => {
                        self.update(PrismEvent::Submit, app_state)
                    }
                    Some((widgets::MenuActionKind::CopyName, name, _)) => copy_and_exit(&name),
                    Some((widgets::MenuActionKind::Effect(effect), _, plugin_id)) => {
                        self.apply_effect(effect, plugin_id)
                    }
                    None => Task::none(),
                }
            }

            PrismEvent::EscapePressed => {
                if !self.state.views.is_empty() {
                    return self.update(PrismEvent::PopView, app_state);
                }
                if self.state.show_actions {
                    self.state.show_actions = false;
                    Task::none()
                } else if self.state.is_argument_input_active {
                    self.state.argument = Option::None;
                    self.state.show_argument_input = false;
                    self.state.is_argument_input_active = false;
                    focus(self.state.search_id.clone())
                } else {
                    Task::done(PrismEvent::ExitApp)
                }
            }

            PrismEvent::GridLeft => {
                if let Some(top) = self.state.views.last_mut() {
                    top.selected = top.selected.saturating_sub(1);
                }
                Task::none()
            }

            PrismEvent::GridRight => {
                if let Some(top) = self.state.views.last_mut() {
                    let len = top.grid_len();
                    if len > 0 {
                        top.selected = (top.selected + 1).min(len - 1);
                    }
                }
                Task::none()
            }

            PrismEvent::ViewSearchInput(text) => {
                if let Some(top) = self.state.views.last_mut() {
                    top.search = text.clone();
                    top.selected = 0;
                }
                self.dispatch_view_event(ViewEventKind::Search(text))
            }

            PrismEvent::ViewItemActivated(id) => {
                self.dispatch_view_event(ViewEventKind::Activate(id))
            }

            PrismEvent::ViewFormText { field_id, value } => {
                if let Some(top) = self.state.views.last_mut() {
                    top.form_values.insert(field_id, FieldValueKind::Text(value));
                }
                Task::none()
            }

            PrismEvent::ViewFormToggle { field_id, value } => {
                if let Some(top) = self.state.views.last_mut() {
                    top.form_values
                        .insert(field_id, FieldValueKind::Toggle(value));
                }
                Task::none()
            }

            PrismEvent::ViewFormChoice { field_id, index } => {
                if let Some(top) = self.state.views.last_mut() {
                    top.form_values
                        .insert(field_id, FieldValueKind::Choice(index));
                }
                Task::none()
            }

            PrismEvent::ViewSubmit => {
                let Some(top) = self.state.views.last() else {
                    return Task::none();
                };

                match &top.view.body {
                    core::ViewBody::Grid { items, .. } => match items.get(top.selected) {
                        Some(item) => {
                            let id = item.id.clone();
                            self.dispatch_view_event(ViewEventKind::Activate(id))
                        }
                        None => Task::none(),
                    },
                    core::ViewBody::Form { .. } => {
                        let values = top.collect_form_values();
                        self.dispatch_view_event(ViewEventKind::Submit(values))
                    }
                    core::ViewBody::Detail { .. } => {
                        self.dispatch_view_event(ViewEventKind::Submit(Vec::new()))
                    }
                }
            }

            PrismEvent::PopView => {
                self.state.views.pop();
                match self.state.views.last() {
                    Some(top) if top.view.search_placeholder.is_some() => {
                        focus(top.search_id.clone())
                    }
                    Some(_) => Task::none(),
                    None => focus(self.state.search_id.clone()),
                }
            }

            PrismEvent::ViewResponse(response) => match response {
                ViewResponse::None => Task::none(),
                ViewResponse::Update(view) => {
                    if let Some(top) = self.state.views.last_mut() {
                        top.view = view;
                        let len = top.grid_len();
                        top.selected = top.selected.min(len.saturating_sub(1));
                    }
                    Task::none()
                }
                ViewResponse::Effect(effect) => {
                    let plugin_id = self
                        .state
                        .views
                        .last()
                        .map(|v| v.plugin_id.clone())
                        .unwrap_or_default();
                    self.apply_effect(effect, plugin_id)
                }
            },

            _ => Task::none(),
        }
    }

    /// Perform a plugin action effect: copy, close, or push a view.
    fn apply_effect(&mut self, effect: ActionEffect, plugin_id: String) -> Task<PrismEvent> {
        match effect {
            ActionEffect::CopyToClipboard(text) => copy_and_exit(&text),
            ActionEffect::Close => Task::done(PrismEvent::ExitApp),
            ActionEffect::PushView(view) => {
                let state = ViewState::new(plugin_id, view);
                let focus_task = state
                    .view
                    .search_placeholder
                    .is_some()
                    .then(|| focus(state.search_id.clone()));
                self.state.views.push(state);
                focus_task.unwrap_or_else(Task::none)
            }
        }
    }

    /// Send a view event to the active view's owning plugin off the UI thread,
    /// delivering the plugin's response as a [`PrismEvent::ViewResponse`].
    fn dispatch_view_event(&self, kind: ViewEventKind) -> Task<PrismEvent> {
        let Some(top) = self.state.views.last() else {
            return Task::none();
        };

        let registry = Arc::clone(&self.registry);
        let plugin_id = top.plugin_id.clone();
        let event = ViewEvent {
            view_id: top.view.view_id.clone(),
            kind,
        };

        Task::perform(
            async move { registry.handle_event(&plugin_id, event) },
            PrismEvent::ViewResponse,
        )
    }

    pub fn get_argument(&self) -> Option<String> {
        self.state.argument.clone()
    }

    pub fn get_selected_entry(&self) -> Option<&PrismEntry> {
        self.state.entries.get(self.state.selected_index)
    }

    pub fn subscription(&self) -> Subscription<PrismEvent> {
        event::listen_with(|event, _status, _window| {
            if let iced::Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) = event
            {
                keybindings::map_key_to_action(&key, modifiers).map(|action| match action {
                    keybindings::KeyAction::SelectPrevious => PrismEvent::SelectPrevious,
                    keybindings::KeyAction::SelectNext => PrismEvent::SelectNext,
                    keybindings::KeyAction::SelectLeft => PrismEvent::GridLeft,
                    keybindings::KeyAction::SelectRight => PrismEvent::GridRight,
                    keybindings::KeyAction::Submit => PrismEvent::Submit,
                    keybindings::KeyAction::EscapePressed => PrismEvent::EscapePressed,
                    keybindings::KeyAction::ToggleActions => PrismEvent::ToggleActions,
                })
            } else {
                None
            }
        })
    }

    /// Number of actions available for the current selection.
    fn actions_len(&self) -> usize {
        self.get_selected_entry()
            .map(|e| widgets::actions_for(&e.entry).len())
            .unwrap_or(0)
    }

    pub fn view<'a>(&'a self) -> Element<'a, PrismEvent> {
        // A plugin view takes over the whole window when one is on the stack.
        if let Some(top) = self.state.views.last() {
            return container(views::view_screen(top))
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(spacing::SPACE_S)
                .style(window_style)
                .into();
        }

        let selected_entry = self.get_selected_entry();

        let content: Element<PrismEvent> = if self.state.show_argument_input
            && let Some(entry) = selected_entry
        {
            column![
                widgets::argument_view(widgets::ArgumentView {
                    command_name: entry.entry.name(),
                    icon: entry.entry.icon(),
                    description: entry.entry.description(),
                    argument_id: self.state.argument_id.clone(),
                    argument: self.state.argument.as_deref(),
                    on_input: Box::new(PrismEvent::ArgumentInput),
                    recent: &self.state.recent_arguments,
                    on_recent: Box::new(PrismEvent::ArgumentSelected),
                }),
                widgets::argument_footer(&entry.entry, self.state.argument.as_deref()),
            ]
            .into()
        } else {
            let search_section = widgets::search_bar(widgets::SearchBar {
                id: self.state.search_id.clone(),
                query: &self.state.query,
                on_input: Box::new(PrismEvent::SearchInput),
                argument_id: self.state.argument_id.clone(),
                argument: self.state.argument.as_deref(),
                on_argument_input: Box::new(PrismEvent::ArgumentInput),
                icon: selected_entry.and_then(|e| e.entry.entity.icon()),
                show_argument_input: false,
            });

            let mut list_section: Vec<Element<PrismEvent>> = Vec::new();
            let mut last_section: Option<&str> = None;
            for (i, entry) in self.state.entries.iter().enumerate() {
                let section = entry.entry.section();
                if last_section != Some(section) {
                    list_section.push(widgets::section_header(section));
                    last_section = Some(section);
                }
                list_section.push(
                    container(widgets::list_item(
                        &entry.entry,
                        i == self.state.selected_index,
                        PrismEvent::EntrySelected(i),
                    ))
                    .id(entry.id.clone())
                    .into(),
                );
            }

            let middle: Element<PrismEvent> = if self.state.entries.is_empty() {
                widgets::empty_state(&self.state.query)
            } else {
                scrollable(column(list_section))
                    .id(self.state.scroll_id.clone())
                    .on_scroll(PrismEvent::Scrolled)
                    .height(Length::Fill)
                    .direction(widgets::slim_scrollbar())
                    .style(widgets::scrollbar_style)
                    .into()
            };

            column![
                search_section,
                widgets::divider(),
                middle,
                widgets::footer(selected_entry.map(|e| &e.entry)),
            ]
            .into()
        };

        let main = container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(spacing::SPACE_S)
        .style(window_style);

        // Overlay the actions popover, anchored bottom-right above the footer.
        if self.state.show_actions
            && let Some(entry) = selected_entry
        {
            let popover = widgets::actions_menu(
                widgets::actions_for(&entry.entry),
                self.state.actions_selected_index,
                PrismEvent::InvokeAction,
            );

            let overlay = container(popover)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(iced::alignment::Horizontal::Right)
                .align_y(iced::alignment::Vertical::Bottom)
                .padding(iced::Padding {
                    top: 0.0,
                    right: 10.0,
                    bottom: 52.0,
                    left: 0.0,
                });

            return iced::widget::stack![main, overlay].into();
        }

        main.into()
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
    ExitApp,
    ToggleActions,
    InvokeAction(usize),
    ArgumentSelected(String),

    // --- Plugin views ---
    GridLeft,
    GridRight,
    ViewSearchInput(String),
    ViewItemActivated(String),
    ViewFormText { field_id: String, value: String },
    ViewFormToggle { field_id: String, value: bool },
    ViewFormChoice { field_id: String, index: u64 },
    ViewSubmit,
    PopView,
    ViewResponse(ViewResponse),
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

/// The launcher window chrome: translucent surface, subtle border, rounded.
fn window_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(colors::SURFACE_CONTAINER.scale_alpha(0.8).into()),
        border: iced::Border {
            color: colors::ON_SURFACE.scale_alpha(0.3),
            width: 1.0,
            radius: 15.0.into(),
        },
        ..Default::default()
    }
}

/// Copy `text` to the clipboard (persisting past exit) and close the launcher.
fn copy_and_exit(text: &str) -> Task<PrismEvent> {
    if let Err(e) = core::clipboard::copy(text) {
        eprintln!("Failed to copy to clipboard: {}", e);
    }
    Task::done(PrismEvent::ExitApp)
}

/// Approximate height of a `section_header` row, used to offset scroll math
/// since headers are not part of the measured `entries`.
const HEADER_HEIGHT: f32 = 26.0;

/// Number of section headers rendered at or above the item at `index`
/// (one per group that starts at or before it).
fn headers_above(state: &PrismState, index: usize) -> usize {
    let mut count = 0;
    let mut last_section: Option<&str> = None;
    for entry in state.entries.iter().take(index + 1) {
        let section = entry.entry.section();
        if last_section != Some(section) {
            count += 1;
            last_section = Some(section);
        }
    }
    count
}

fn smart_scroll(state: &PrismState) -> Task<PrismEvent> {
    let mut y_position = headers_above(state, state.selected_index) as f32 * HEADER_HEIGHT;
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
