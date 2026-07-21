mod items;
mod keybindings;
pub mod state;
mod views;
mod widgets;

use std::sync::Arc;

use self::state::{ImageEntry, PrismEntry, PrismState, ViewState};
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
            image_cache: std::collections::HashMap::new(),
            anim_epoch: std::time::Instant::now(),
            anim_now: std::time::Instant::now(),
        };

        let registry = Arc::new(PluginRegistry::with_builtins());

        let load_registry = Arc::clone(&registry);
        let load_task = Task::perform(
            async move {
                get_entities(&load_registry)
                    .into_iter()
                    .map(From::from)
                    .collect::<Vec<_>>()
            },
            PrismEvent::EntriesLoaded,
        );
        let init_task = Task::perform(async {}, |_| PrismEvent::Initialized);

        (
            Self { state, registry },
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
                        // Fallback commands are only offered at the bottom.
                        .filter(|e| !e.entry.entity.is_fallback_command())
                        .filter(|e| SearchEngine::matches(&e.search_haystack, &query_lower))
                        .cloned(),
                );

                // Offer fallback commands (e.g. "Search Google") on whatever the
                // user typed, so they can always act on the raw query.
                if !self.state.query.is_empty() {
                    for candidate in &self.state.all_entries {
                        if let Some(entity) = candidate.entry.entity.fallback_for(&self.state.query)
                        {
                            entries.push(PrismEntry::from(ListEntry::from(entity)));
                        }
                    }
                }

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
                    return self.grid_nav_tasks();
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
                    return self.grid_nav_tasks();
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
                let selection = self.get_selected_entry().map(|entry| {
                    let entity = &entry.entry.entity;
                    (
                        entity.needs_argument(),
                        entity.name().to_string(),
                        entity
                            .command_ref()
                            .map(|(p, c)| (p.to_string(), c.to_string())),
                        entity.fallback_query().map(str::to_string),
                    )
                });

                let Some((needs_argument, name, command, fallback_query)) = selection else {
                    return Task::none();
                };

                // Commands (and apps) that take an argument open the input first.
                if needs_argument && self.get_argument().is_none() {
                    self.state.show_argument_input = true;
                    self.state.is_argument_input_active = true;
                    self.state.recent_arguments = app_state.recent_arguments(&name);
                    return focus(self.state.argument_id.clone());
                }

                self.state.is_argument_input_active = false;

                // Plugin command: record usage, run it, and apply the effect.
                if let Some((plugin_id, command_id)) = command {
                    // A fallback runs on the typed query; a normal command uses
                    // the argument input.
                    let argument = fallback_query.or_else(|| self.get_argument());
                    if let Some(entry) = self.get_selected_entry() {
                        app_state.record_usage(&entry.entry.entity);
                    }
                    if let Some(arg) = argument.as_deref().filter(|a| !a.is_empty()) {
                        app_state.record_argument(&name, arg);
                    }
                    if let Err(e) = app_state.save() {
                        eprintln!("Failed to save state: {e}");
                    }

                    let effect =
                        self.registry
                            .run_command(&plugin_id, &command_id, argument.as_deref());
                    return self.apply_effect(effect, plugin_id);
                }

                // Application: launch via the Run path.
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
                    let plugin_id = entry
                        .entry
                        .entity
                        .plugin_source_id()
                        .unwrap_or_default()
                        .to_string();
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
                self.grid_nav_tasks()
            }

            PrismEvent::GridRight => {
                if let Some(top) = self.state.views.last_mut() {
                    let len = top.grid_len();
                    if len > 0 {
                        top.selected = (top.selected + 1).min(len - 1);
                    }
                }
                self.grid_nav_tasks()
            }

            PrismEvent::ViewSearchInput(text) => {
                let already_searching = match self.state.views.last_mut() {
                    Some(top) => {
                        top.search = text.clone();
                        top.selected = 0;
                        let busy = top.searching;
                        if busy {
                            // Coalesce: only keep the most recent term to run next.
                            top.pending_search = Some(text.clone());
                        } else {
                            top.searching = true;
                        }
                        busy
                    }
                    None => return Task::none(),
                };

                if already_searching {
                    Task::none()
                } else {
                    self.dispatch_search(text)
                }
            }

            PrismEvent::ViewItemActivated(id) => {
                self.dispatch_view_event(ViewEventKind::Activate(id))
            }

            PrismEvent::ViewFormText { field_id, value } => {
                if let Some(top) = self.state.views.last_mut() {
                    top.form_values
                        .insert(field_id, FieldValueKind::Text(value));
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

            PrismEvent::ViewScrolled(viewport) => {
                if let Some(top) = self.state.views.last_mut() {
                    top.scroll_offset = viewport.absolute_offset().y;
                    top.viewport_height = viewport.bounds().height;
                }

                // Load the next page when scrolled near the bottom.
                let should_load = match self.state.views.last() {
                    Some(top) => {
                        viewport.relative_offset().y > 0.75
                            && top.more_available
                            && !top.loading_more
                            && !top.searching
                            && matches!(top.view.body, core::ViewBody::Grid { .. })
                    }
                    None => false,
                };

                if !should_load {
                    return Task::none();
                }

                let (term, offset) = {
                    let top = self.state.views.last_mut().unwrap();
                    top.loading_more = true;
                    (top.search.clone(), top.loaded_count as u64)
                };

                self.dispatch_view_event(ViewEventKind::LoadMore { term, offset })
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

            PrismEvent::ViewResponse(response) => self.apply_view_response(response),

            PrismEvent::SearchCompleted { term, response } => {
                let apply = self.apply_view_response(response);

                // Run the most recent queued term, if it differs from this one.
                let next = self.state.views.last_mut().and_then(|top| {
                    top.searching = false;
                    top.pending_search.take().filter(|pending| *pending != term)
                });

                match next {
                    Some(pending) => {
                        if let Some(top) = self.state.views.last_mut() {
                            top.searching = true;
                        }
                        Task::batch(vec![apply, self.dispatch_search(pending)])
                    }
                    None => apply,
                }
            }

            PrismEvent::ImageLoaded { url, frames } => {
                let entry = match frames {
                    Some(frames) if !frames.is_empty() => ImageEntry::Loaded(frames),
                    _ => ImageEntry::Failed,
                };
                self.state.image_cache.insert(url, entry);
                Task::none()
            }

            PrismEvent::AnimationTick(now) => {
                self.state.anim_now = now;
                Task::none()
            }

            PrismEvent::GridRowMeasured { id, height } => {
                if height > 0.0
                    && let Some(top) = self.state.views.last_mut()
                {
                    top.row_heights.insert(id, height);
                }
                Task::none()
            }

            _ => Task::none(),
        }
    }

    /// Apply a plugin's view response: replace the view (loading its images),
    /// perform an effect, or nothing.
    fn apply_view_response(&mut self, response: ViewResponse) -> Task<PrismEvent> {
        match response {
            ViewResponse::None => Task::none(),
            ViewResponse::Update(view) => {
                let mut reset_scroll = Task::none();
                if let Some(top) = self.state.views.last_mut() {
                    top.view = view;
                    let len = top.grid_len();
                    top.selected = top.selected.min(len.saturating_sub(1));
                    // New result set: reset pagination and scroll to the top.
                    top.loaded_count = len;
                    top.loading_more = false;
                    top.more_available = true;
                    top.scroll_offset = 0.0;
                    reset_scroll = scroll_to(
                        top.scroll_id.clone(),
                        scrollable::AbsoluteOffset { x: 0.0, y: 0.0 },
                    );
                }
                Task::batch(vec![
                    reset_scroll,
                    self.sync_grid_rows(true),
                    self.ensure_grid_images(),
                ])
            }
            ViewResponse::Append(items) => {
                if let Some(top) = self.state.views.last_mut() {
                    top.loading_more = false;
                    if items.is_empty() {
                        top.more_available = false;
                    } else if let core::ViewBody::Grid {
                        items: grid_items, ..
                    } = &mut top.view.body
                    {
                        grid_items.extend(items);
                        top.loaded_count = grid_items.len();
                    }
                }
                Task::batch(vec![self.sync_grid_rows(false), self.ensure_grid_images()])
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
        }
    }

    /// Scroll the active grid so the selected cell stays visible under keyboard
    /// navigation, using measured row heights (same approach as the list).
    fn scroll_grid_to_selected(&mut self) -> Task<PrismEvent> {
        // Vertical gap between grid rows (matches `spacing::SPACE_M`) and the
        // scrollable's top padding (`spacing::SPACE_S`).
        const ROW_SPACING: f32 = 16.0;
        const TOP_PAD: f32 = 8.0;
        const FALLBACK_ROW: f32 = 124.0;

        let Some(top) = self.state.views.last_mut() else {
            return Task::none();
        };
        if !matches!(top.view.body, core::ViewBody::Grid { .. }) {
            return Task::none();
        }

        let selected_row = top.selected / top.grid_columns();
        // A measured row height to fall back to for rows not yet measured.
        let default_row = top
            .row_heights
            .values()
            .copied()
            .next()
            .unwrap_or(FALLBACK_ROW);
        let height_of = |index: usize| {
            top.row_ids
                .get(index)
                .and_then(|id| top.row_heights.get(id))
                .copied()
                .unwrap_or(default_row)
        };

        let mut row_top = TOP_PAD;
        for row in 0..selected_row {
            row_top += height_of(row) + ROW_SPACING;
        }
        let row_bottom = row_top + height_of(selected_row);

        let viewport = if top.viewport_height > 0.0 {
            top.viewport_height
        } else {
            340.0
        };
        let view_top = top.scroll_offset;
        let view_bottom = view_top + viewport;

        let target = if row_top < view_top {
            Some(row_top)
        } else if row_bottom > view_bottom {
            Some(row_bottom - viewport)
        } else {
            None
        };

        match target {
            Some(y) => {
                top.scroll_offset = y;
                scroll_to(
                    top.scroll_id.clone(),
                    scrollable::AbsoluteOffset { x: 0.0, y },
                )
            }
            None => Task::none(),
        }
    }

    /// Refresh the per-row ids for the active grid and measure their heights.
    /// `reset` regenerates all ids (new search); otherwise ids are extended for
    /// appended rows.
    fn sync_grid_rows(&mut self, reset: bool) -> Task<PrismEvent> {
        let Some(top) = self.state.views.last_mut() else {
            return Task::none();
        };
        let core::ViewBody::Grid { items, columns } = &top.view.body else {
            return Task::none();
        };

        let columns = (*columns as usize).max(1);
        let rows = items.len().div_ceil(columns);

        if reset {
            top.row_ids.clear();
            top.row_heights.clear();
        }
        while top.row_ids.len() < rows {
            top.row_ids.push(Id::unique());
        }
        top.row_ids.truncate(rows);

        measure_grid_rows(&top.row_ids)
    }

    /// Combined keyboard-nav response for a grid: keep the selection visible and
    /// fetch the next page when navigating into the last loaded row.
    fn grid_nav_tasks(&mut self) -> Task<PrismEvent> {
        Task::batch(vec![
            self.scroll_grid_to_selected(),
            self.maybe_paginate_selection(),
        ])
    }

    /// Trigger pagination if the selection has reached the last loaded row.
    fn maybe_paginate_selection(&mut self) -> Task<PrismEvent> {
        let request = {
            let Some(top) = self.state.views.last() else {
                return Task::none();
            };
            let near_end = top.selected + top.grid_columns() >= top.loaded_count;
            let can_load = matches!(top.view.body, core::ViewBody::Grid { .. })
                && near_end
                && top.more_available
                && !top.loading_more
                && !top.searching
                && top.loaded_count > 0;
            can_load.then(|| (top.search.clone(), top.loaded_count as u64))
        };

        match request {
            Some((term, offset)) => {
                if let Some(top) = self.state.views.last_mut() {
                    top.loading_more = true;
                }
                self.dispatch_view_event(ViewEventKind::LoadMore { term, offset })
            }
            None => Task::none(),
        }
    }

    /// Kick off async fetches for any not-yet-cached grid image URLs in the
    /// active view.
    fn ensure_grid_images(&mut self) -> Task<PrismEvent> {
        let Some(top) = self.state.views.last() else {
            return Task::none();
        };
        let core::ViewBody::Grid { items, .. } = &top.view.body else {
            return Task::none();
        };

        let urls: Vec<String> = items
            .iter()
            .filter_map(|item| match &item.image {
                core::ImageSource::Url(url) if !self.state.image_cache.contains_key(url) => {
                    Some(url.clone())
                }
                _ => None,
            })
            .collect();

        let mut tasks = Vec::new();
        for url in urls {
            self.state
                .image_cache
                .insert(url.clone(), ImageEntry::Loading);

            let fetch_url = url.clone();
            tasks.push(Task::perform(
                async move { core::net::fetch_bytes(&fetch_url).ok().map(build_frames) },
                move |frames| PrismEvent::ImageLoaded {
                    url: url.clone(),
                    frames,
                },
            ));
        }

        Task::batch(tasks)
    }

    /// Dispatch a search to the active view's plugin, tagging the response with
    /// the term so stale/queued searches can be reconciled.
    fn dispatch_search(&self, term: String) -> Task<PrismEvent> {
        let Some(top) = self.state.views.last() else {
            return Task::none();
        };

        let registry = Arc::clone(&self.registry);
        let plugin_id = top.plugin_id.clone();
        let view_id = top.view.view_id.clone();
        let search_term = term.clone();

        Task::perform(
            async move {
                registry.handle_event(
                    &plugin_id,
                    ViewEvent {
                        view_id,
                        kind: ViewEventKind::Search(search_term),
                    },
                )
            },
            move |response| PrismEvent::SearchCompleted {
                term: term.clone(),
                response,
            },
        )
    }

    /// Perform a plugin action effect: copy, close, or push a view.
    fn apply_effect(&mut self, effect: ActionEffect, plugin_id: String) -> Task<PrismEvent> {
        match effect {
            ActionEffect::None => Task::none(),
            ActionEffect::CopyToClipboard(text) => copy_and_exit(&text),
            ActionEffect::OpenUrl(url) => {
                if let Err(e) = core::open::url(&url) {
                    eprintln!("Failed to open URL: {e}");
                }
                Task::done(PrismEvent::ExitApp)
            }
            ActionEffect::Close => Task::done(PrismEvent::ExitApp),
            ActionEffect::PushView(view) => {
                let mut state = ViewState::new(plugin_id, view);
                let has_search = state.view.search_placeholder.is_some();
                let is_grid = matches!(state.view.body, core::ViewBody::Grid { .. });
                let search_id = state.search_id.clone();
                if is_grid && has_search {
                    state.searching = true;
                }
                self.state.views.push(state);

                let mut tasks = Vec::new();
                if has_search {
                    tasks.push(focus(search_id));
                }
                // Load any images already present, then (for searchable grids)
                // fetch the initial contents by handling an empty search.
                tasks.push(self.ensure_grid_images());
                if is_grid && has_search {
                    tasks.push(self.dispatch_search(String::new()));
                }
                Task::batch(tasks)
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
        let keys = event::listen_with(|event, _status, _window| {
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
        });

        // Only drive animation frames while a grid with animated GIFs is shown.
        if self.has_animated_grid() {
            Subscription::batch([keys, iced::window::frames().map(PrismEvent::AnimationTick)])
        } else {
            keys
        }
    }

    /// Whether the active view is a grid displaying at least one animated image.
    fn has_animated_grid(&self) -> bool {
        let Some(top) = self.state.views.last() else {
            return false;
        };
        let core::ViewBody::Grid { items, .. } = &top.view.body else {
            return false;
        };
        items.iter().any(|item| match &item.image {
            core::ImageSource::Url(url) => self
                .state
                .image_cache
                .get(url)
                .is_some_and(ImageEntry::is_animated),
            _ => false,
        })
    }

    /// Elapsed animation time in milliseconds.
    fn anim_elapsed_ms(&self) -> u64 {
        self.state
            .anim_now
            .saturating_duration_since(self.state.anim_epoch)
            .as_millis() as u64
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
            return container(views::view_screen(
                top,
                &self.state.image_cache,
                self.anim_elapsed_ms(),
            ))
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
    ItemMeasured {
        id: Id,
        rect: Rectangle,
    },
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
    ViewFormText {
        field_id: String,
        value: String,
    },
    ViewFormToggle {
        field_id: String,
        value: bool,
    },
    ViewFormChoice {
        field_id: String,
        index: u64,
    },
    ViewSubmit,
    ViewScrolled(scrollable::Viewport),
    PopView,
    ViewResponse(ViewResponse),
    SearchCompleted {
        term: String,
        response: ViewResponse,
    },
    ImageLoaded {
        url: String,
        frames: Option<Vec<(iced::widget::image::Handle, u32)>>,
    },
    AnimationTick(std::time::Instant),
    GridRowMeasured {
        id: Id,
        height: f32,
    },
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

/// Measure each grid row's height (by its container id) into `GridRowMeasured`
/// events — the grid analogue of [`measure_item`].
fn measure_grid_rows(row_ids: &[Id]) -> Task<PrismEvent> {
    let tasks: Vec<Task<PrismEvent>> = row_ids
        .iter()
        .map(|id| {
            let id = id.clone();
            let selector = selector::id(id.clone()).find();
            let operation = operation::map(selector, move |v| PrismEvent::GridRowMeasured {
                id: id.clone(),
                height: v.map(|widget| widget.bounds().height).unwrap_or(0.0),
            });
            operate(operation)
        })
        .collect();

    Task::batch(tasks)
}

/// Decode fetched image bytes into displayable frames. Handles are built once
/// here (off the UI thread) so re-renders reuse the GPU textures.
fn build_frames(bytes: Vec<u8>) -> Vec<(iced::widget::image::Handle, u32)> {
    use iced::widget::image::Handle;
    match core::media::decode(bytes) {
        core::media::Decoded::Still(bytes) => vec![(Handle::from_bytes(bytes), 0)],
        core::media::Decoded::Animated(frames) => frames
            .into_iter()
            .map(|frame| {
                (
                    Handle::from_rgba(frame.width, frame.height, frame.rgba),
                    frame.delay_ms,
                )
            })
            .collect(),
    }
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
