//! The Plugin Manager — a full-window "Settings" surface that lists installed
//! plugins (master) alongside the selected plugin's details (detail): its
//! preferences, the commands it provides, metadata and an uninstall flow.
//!
//! Opened with ⌘/Ctrl+`,` and dismissed with `Esc`. The plugin list and each
//! plugin's commands are read from the live [`PluginRegistry`]; richer metadata
//! (author, version, description, preferences) is filled in host-side per known
//! plugin, since the plugin ABI does not carry it. Enable/preference toggles and
//! uninstall are session-local (they mutate only this screen's state) — the same
//! behaviour the source design models.

use core::{AppState, PluginCommand, PluginRegistry, Preference, PreferenceKind, PreferenceValue};

use iced::{
    Alignment, Color, Element, Length,
    widget::{
        Id, button, column, container, pick_list, row, scrollable, space::horizontal, text,
        text_input,
    },
};

use super::PrismEvent;
use super::widgets::{scrollbar_style, slim_scrollbar};
use crate::design_system::{colors, spacing, typo};

// --- Settings-window palette (matched to the source design) -----------------

const WINDOW_BG: Color = Color::from_rgb8(0x1c, 0x1c, 0x1f);
const TITLEBAR_BG: Color = Color::from_rgb8(0x2a, 0x2a, 0x2e);
const TABBAR_BG: Color = Color::from_rgb8(0x23, 0x23, 0x26);
const MASTER_BG: Color = Color::from_rgb8(0x20, 0x20, 0x23);
const CARD_BG: Color = Color::from_rgb8(0x24, 0x24, 0x27);
const TITLEBAR_FG: Color = Color::from_rgb8(0xc7, 0xc7, 0xcc);
const DESC_FG: Color = Color::from_rgb8(0xd8, 0xd8, 0xdc);
const DANGER_FG: Color = Color::from_rgb8(0xff, 0x7a, 0x81);
const LIGHT_RED: Color = Color::from_rgb8(0xff, 0x5f, 0x57);
const LIGHT_YELLOW: Color = Color::from_rgb8(0xfe, 0xbc, 0x2e);
const LIGHT_GREEN: Color = Color::from_rgb8(0x28, 0xc8, 0x40);

/// A translucent-black hairline (`rgba(0,0,0,0.4)`), used for chrome borders.
const HAIRLINE: Color = Color {
    r: 0.0,
    g: 0.0,
    b: 0.0,
    a: 0.4,
};

/// A faint light hairline used for interior dividers.
fn soft_line() -> Color {
    colors::ON_SURFACE.scale_alpha(0.06)
}

// --- Model ------------------------------------------------------------------

/// One installed plugin as shown in the manager.
#[derive(Clone)]
pub struct PmPlugin {
    pub id: String,
    pub name: String,
    pub letter: char,
    pub color: Color,
    pub ink: Color,
    pub author: String,
    pub version: String,
    pub updated: String,
    pub size: String,
    pub desc: String,
    pub prefs: Vec<PmPref>,
    pub commands: Vec<PmCommand>,
    pub enabled: bool,
}

/// A single preference row for a plugin.
#[derive(Clone)]
pub struct PmPref {
    /// Stable preference id (used to persist and notify the plugin).
    pub id: String,
    pub label: String,
    pub hint: String,
    pub control: PmControl,
}

/// The control rendered on the right of a preference row.
#[derive(Clone)]
pub enum PmControl {
    /// An interactive on/off switch.
    Toggle(bool),
    /// A cycle-on-click dropdown: options and the selected index.
    Select {
        options: Vec<String>,
        selected: usize,
    },
    /// A (display-only) free-text field.
    Text(String),
    /// A (display-only) masked field, rendered monospace.
    Secret(String),
}

impl PmControl {
    /// Advance this control to its next state (flip a toggle, cycle to the next
    /// option) and return the new value to persist, or `None` if it is not an
    /// interactive control.
    pub fn activate(&mut self) -> Option<PreferenceValue> {
        match self {
            PmControl::Toggle(on) => {
                *on = !*on;
                Some(PreferenceValue::Toggle(*on))
            }
            PmControl::Select { options, selected } => {
                if options.is_empty() {
                    return None;
                }
                *selected = (*selected + 1) % options.len();
                Some(PreferenceValue::Choice(*selected as u64))
            }
            PmControl::Text(_) | PmControl::Secret(_) => None,
        }
    }

    /// Set a select control to a specific option index and return the value to
    /// persist, or `None` if this isn't a select or the index is out of range.
    pub fn set_selected(&mut self, option: usize) -> Option<PreferenceValue> {
        match self {
            PmControl::Select { options, selected } if option < options.len() => {
                *selected = option;
                Some(PreferenceValue::Choice(option as u64))
            }
            _ => None,
        }
    }

    /// Set a text/secret control's value and return the value to persist, or
    /// `None` if this isn't a text control.
    pub fn set_text(&mut self, value: String) -> Option<PreferenceValue> {
        match self {
            PmControl::Text(current) | PmControl::Secret(current) => {
                *current = value.clone();
                Some(PreferenceValue::Text(value))
            }
            _ => None,
        }
    }
}

/// A command a plugin provides, as shown in the detail pane.
#[derive(Clone)]
pub struct PmCommand {
    pub glyph: char,
    pub color: Color,
    pub ink: Color,
    pub name: String,
    pub desc: String,
    /// A bound hotkey (rendered as a chip) or `None` (an unbound "Record Hotkey").
    pub hotkey: Option<String>,
}

/// Interactions within the Plugin Manager.
#[derive(Debug, Clone)]
pub enum PmEvent {
    /// Show a plugin's details.
    Select(String),
    /// Flip a plugin's enabled switch.
    ToggleEnabled(String),
    /// Advance an interactive preference control (`index` into that plugin's
    /// `prefs`): flip a toggle. Persists and notifies.
    ActivatePref { plugin: String, index: usize },
    /// Set a select preference (`index`) to a chosen `option`. Persists and
    /// notifies.
    SelectPref {
        plugin: String,
        index: usize,
        option: usize,
    },
    /// Edit a text/secret preference (`index`) to `value`. Persists and notifies.
    EditPref {
        plugin: String,
        index: usize,
        value: String,
    },
    /// The master-list search text changed.
    Search(String),
    /// Open the uninstall confirmation.
    UninstallRequest,
    /// Dismiss the uninstall confirmation.
    UninstallCancel,
    /// Uninstall the selected plugin.
    UninstallConfirm,
    /// Close the manager and return to the launcher.
    Close,
}

/// Live state of the Plugin Manager screen.
pub struct PluginManagerState {
    pub plugins: Vec<PmPlugin>,
    pub selected_id: Option<String>,
    pub search: String,
    pub search_id: Id,
    pub confirming: bool,
}

impl PluginManagerState {
    /// Build the manager's state from the live registry, overlaying persisted
    /// preference values from `app_state`.
    pub fn new(registry: &PluginRegistry, app_state: &AppState) -> Self {
        let plugins = build_plugins(registry, app_state);
        let selected_id = plugins.first().map(|p| p.id.clone());
        Self {
            plugins,
            selected_id,
            search: String::new(),
            search_id: Id::unique(),
            confirming: false,
        }
    }

    /// The plugin whose details are shown: the selected one, or the first.
    fn selected(&self) -> Option<&PmPlugin> {
        self.plugins
            .iter()
            .find(|p| Some(&p.id) == self.selected_id.as_ref())
            .or_else(|| self.plugins.first())
    }

    /// Plugins matching the current search filter (case-insensitive on name).
    fn filtered(&self) -> Vec<&PmPlugin> {
        let needle = self.search.trim().to_lowercase();
        self.plugins
            .iter()
            .filter(|p| needle.is_empty() || p.name.to_lowercase().contains(&needle))
            .collect()
    }
}

// --- Building the model from the registry -----------------------------------

fn build_plugins(registry: &PluginRegistry, app_state: &AppState) -> Vec<PmPlugin> {
    let all_commands = registry.commands();

    registry
        .plugin_ids()
        .into_iter()
        .map(|id| {
            // Metadata and preferences are declared by the plugin itself (over
            // the ABI); the host only fills gaps and derives install facts.
            let meta = registry.metadata(&id).unwrap_or_default();
            let commands: Vec<PmCommand> = all_commands
                .iter()
                .filter(|(plugin_id, _)| *plugin_id == id)
                .map(|(_, command)| command_from(command))
                .collect();
            let prefs: Vec<PmPref> = registry
                .preferences(&id)
                .into_iter()
                // Overlay the user's persisted value on the declared default.
                .map(|pref| {
                    let current = app_state.preference(&id, &pref.id);
                    pref_from(pref, current)
                })
                .collect();

            let name = meta.name.unwrap_or_else(|| prettify_id(&id));
            let (updated, size) = match registry.install_info(&id) {
                Some(info) => (format_modified(info.modified), format_size(info.size_bytes)),
                None => ("bundled".to_string(), "—".to_string()),
            };

            let color = colors::tile_color(&name);
            let letter = name.chars().next().unwrap_or('?').to_ascii_uppercase();

            PmPlugin {
                id,
                name,
                letter,
                color,
                ink: ink_for(color),
                author: meta.author.unwrap_or_else(|| "unknown".to_string()),
                version: meta.version.unwrap_or_else(|| "—".to_string()),
                updated,
                size,
                desc: meta.description.unwrap_or_else(|| {
                    "An installed plugin. It provides the commands listed below.".to_string()
                }),
                prefs,
                commands,
                enabled: true,
            }
        })
        .collect()
}

/// Map a plugin-declared [`Preference`] onto the manager's row model, using the
/// user's persisted `current` value in place of the declared default when set.
fn pref_from(pref: Preference, current: Option<PreferenceValue>) -> PmPref {
    let control = match pref.kind {
        PreferenceKind::Toggle(default) => {
            let on = match current {
                Some(PreferenceValue::Toggle(on)) => on,
                _ => default,
            };
            PmControl::Toggle(on)
        }
        PreferenceKind::Select { options, selected } => {
            let selected = match current {
                Some(PreferenceValue::Choice(index)) => index as usize,
                _ => selected as usize,
            };
            let selected = selected.min(options.len().saturating_sub(1));
            PmControl::Select { options, selected }
        }
        PreferenceKind::Text(default) => {
            let value = match current {
                Some(PreferenceValue::Text(value)) => value,
                _ => default,
            };
            PmControl::Text(value)
        }
        PreferenceKind::Secret(default) => {
            let value = match current {
                Some(PreferenceValue::Text(value)) => value,
                _ => default,
            };
            PmControl::Secret(value)
        }
    };
    PmPref {
        id: pref.id,
        label: pref.label,
        hint: pref.hint,
        control,
    }
}

/// Format a library file size for the metadata cell (e.g. "1.2 MB").
fn format_size(bytes: u64) -> String {
    const MB: f64 = 1024.0 * 1024.0;
    const KB: f64 = 1024.0;
    let b = bytes as f64;
    if b >= MB {
        format!("{:.1} MB", b / MB)
    } else if b >= KB {
        format!("{:.0} KB", b / KB)
    } else {
        format!("{bytes} B")
    }
}

/// Format a file's modified time as a coarse "N units ago" string.
fn format_modified(modified: Option<std::time::SystemTime>) -> String {
    let Some(time) = modified else {
        return "—".to_string();
    };
    let Ok(elapsed) = std::time::SystemTime::now().duration_since(time) else {
        return "just now".to_string();
    };
    let secs = elapsed.as_secs();
    let (count, unit) = if secs < 60 {
        return "just now".to_string();
    } else if secs < 3_600 {
        (secs / 60, "minute")
    } else if secs < 86_400 {
        (secs / 3_600, "hour")
    } else if secs < 2_592_000 {
        (secs / 86_400, "day")
    } else if secs < 31_536_000 {
        (secs / 2_592_000, "month")
    } else {
        (secs / 31_536_000, "year")
    };
    let plural = if count == 1 { "" } else { "s" };
    format!("{count} {unit}{plural} ago")
}

fn command_from(command: &PluginCommand) -> PmCommand {
    let glyph = command.glyph.unwrap_or_else(|| {
        command
            .title
            .chars()
            .next()
            .unwrap_or('•')
            .to_ascii_uppercase()
    });
    let color = colors::tile_color(&command.title);
    PmCommand {
        glyph,
        color,
        ink: ink_for(color),
        name: command.title.clone(),
        desc: command
            .subtitle
            .clone()
            .unwrap_or_else(|| command.category.clone()),
        // The registry carries no bound hotkeys, so every command is offered as
        // an unbound "Record Hotkey" affordance.
        hotkey: None,
    }
}

/// Turn a plugin id such as `web.google` into a display name like `Web Google`.
fn prettify_id(id: &str) -> String {
    id.split(['.', '_', '-', ' '])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// A readable ink color (near-black on bright tiles, white on dark ones).
fn ink_for(color: Color) -> Color {
    let luminance = 0.299 * color.r + 0.587 * color.g + 0.114 * color.b;
    if luminance > 0.6 {
        Color::from_rgb8(0x12, 0x12, 0x16)
    } else {
        Color::WHITE
    }
}

// --- View -------------------------------------------------------------------

/// Render the whole settings window (its own chrome, filling the launcher).
pub fn view(state: &PluginManagerState) -> Element<'_, PrismEvent> {
    let window = column![
        title_bar(),
        chrome_line(),
        tab_bar(),
        chrome_line(),
        body(state),
    ];

    let chrome = container(window)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_| container::Style {
            background: Some(WINDOW_BG.into()),
            border: iced::Border {
                color: colors::ON_SURFACE.scale_alpha(0.12),
                width: 1.0,
                radius: 14.0.into(),
            },
            ..Default::default()
        })
        .clip(true);

    // The uninstall confirmation floats over a dimmed window.
    if state.confirming {
        if let Some(sel) = state.selected() {
            return iced::widget::stack![chrome, uninstall_overlay(sel)].into();
        }
    }

    chrome.into()
}

fn title_bar<'a>() -> Element<'a, PrismEvent> {
    // The red light closes the manager (Esc does too); the others are inert.
    let lights = row![
        close_light(),
        traffic_light(LIGHT_YELLOW),
        traffic_light(LIGHT_GREEN),
    ]
    .spacing(spacing::SPACE_S)
    .align_y(Alignment::Center);

    let bar = row![
        lights,
        container(
            text("Settings")
                .size(13.0)
                .font(typo::TITLE_M.2)
                .color(TITLEBAR_FG)
        )
        .center_x(Length::Fill),
        // Balances the traffic lights so the title stays optically centered.
        container(text("")).width(Length::Fixed(52.0)),
    ]
    .align_y(Alignment::Center)
    .spacing(spacing::SPACE_S);

    container(bar)
        .width(Length::Fill)
        .center_y(Length::Fixed(44.0))
        .padding(iced::Padding {
            top: 0.0,
            right: 16.0,
            bottom: 0.0,
            left: 16.0,
        })
        .style(|_| container::Style {
            background: Some(TITLEBAR_BG.into()),
            ..Default::default()
        })
        .into()
}

/// The clickable red traffic light that closes the manager.
fn close_light<'a>() -> Element<'a, PrismEvent> {
    button(
        container(text(""))
            .width(Length::Fixed(12.0))
            .height(Length::Fixed(12.0)),
    )
    .padding(0.0)
    .on_press(PrismEvent::PluginManager(PmEvent::Close))
    .style(|_, status| {
        let hovered = status == button::Status::Hovered;
        button::Style {
            background: Some(
                if hovered {
                    LIGHT_RED.scale_alpha(0.85)
                } else {
                    LIGHT_RED
                }
                .into(),
            ),
            border: iced::Border {
                radius: 6.0.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    })
    .into()
}

fn traffic_light<'a>(color: Color) -> Element<'a, PrismEvent> {
    container(text(""))
        .width(Length::Fixed(12.0))
        .height(Length::Fixed(12.0))
        .style(move |_| container::Style {
            background: Some(color.into()),
            border: iced::Border {
                radius: 6.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}

fn tab_bar<'a>() -> Element<'a, PrismEvent> {
    // Tabs are presentational (as in the source design); Extensions is active.
    let tabs = row![
        tab_item("⚙", "General", false),
        tab_item("▦", "Extensions", true),
        tab_item("ⓘ", "About", false),
    ]
    .spacing(2.0)
    .align_y(Alignment::Center);

    container(tabs)
        .width(Length::Fill)
        .padding(iced::Padding {
            top: 8.0,
            right: 12.0,
            bottom: 8.0,
            left: 12.0,
        })
        .style(|_| container::Style {
            background: Some(TABBAR_BG.into()),
            ..Default::default()
        })
        .into()
}

fn tab_item<'a>(icon: &str, label: &str, active: bool) -> Element<'a, PrismEvent> {
    let fg = if active {
        colors::ON_SURFACE
    } else {
        colors::SECONDARY
    };

    let content = column![
        text(icon.to_string()).size(16.0).color(fg),
        text(label.to_string())
            .size(11.0)
            .font(typo::LABEL_S.2)
            .color(fg),
    ]
    .spacing(3.0)
    .align_x(Alignment::Center);

    container(content)
        .center_x(Length::Fixed(74.0))
        .padding(iced::Padding {
            top: 6.0,
            right: 14.0,
            bottom: 6.0,
            left: 14.0,
        })
        .style(move |_| container::Style {
            background: active.then(|| colors::ON_SURFACE.scale_alpha(0.1).into()),
            border: iced::Border {
                radius: 8.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}

fn body(state: &PluginManagerState) -> Element<'_, PrismEvent> {
    row![master(state), vline(Length::Fill), detail(state)]
        .height(Length::Fill)
        .into()
}

// --- Master (installed plugin list) -----------------------------------------

fn master(state: &PluginManagerState) -> Element<'_, PrismEvent> {
    let filtered = state.filtered();

    let search = container(
        row![
            text("⌕").size(14.0).color(colors::SECONDARY),
            text_input("Search plugins…", &state.search)
                .id(state.search_id.clone())
                .on_input(|value| PrismEvent::PluginManager(PmEvent::Search(value)))
                .size(13.0)
                .padding(0.0)
                .style(|_, _| text_input::Style {
                    background: Color::TRANSPARENT.into(),
                    border: iced::Border::default(),
                    icon: Color::WHITE,
                    placeholder: colors::SECONDARY,
                    value: colors::ON_SURFACE,
                    selection: colors::ON_SURFACE.scale_alpha(0.3),
                }),
        ]
        .spacing(spacing::SPACE_S)
        .align_y(Alignment::Center),
    )
    .padding(iced::Padding {
        top: 7.0,
        right: 10.0,
        bottom: 7.0,
        left: 10.0,
    })
    .style(|_| container::Style {
        background: Some(colors::ON_SURFACE.scale_alpha(0.06).into()),
        border: iced::Border {
            color: colors::ON_SURFACE.scale_alpha(0.08),
            width: 1.0,
            radius: 8.0.into(),
        },
        ..Default::default()
    });

    let selected_id = state.selected().map(|p| p.id.clone());
    let mut list = column![
        container(
            text(format!("Installed · {}", state.plugins.len()))
                .size(11.0)
                .font(typo::LABEL_S.2)
                .color(colors::SECONDARY),
        )
        .padding(iced::Padding {
            top: 6.0,
            right: 8.0,
            bottom: 4.0,
            left: 8.0,
        })
    ]
    .spacing(1.0);

    for plugin in filtered {
        let active = Some(&plugin.id) == selected_id.as_ref();
        list = list.push(master_row(plugin, active));
    }

    let scroll = scrollable(list)
        .height(Length::Fill)
        .direction(slim_scrollbar())
        .style(scrollbar_style);

    let column = column![
        container(search).padding(iced::Padding {
            top: 12.0,
            right: 12.0,
            bottom: 8.0,
            left: 12.0,
        }),
        container(scroll)
            .height(Length::Fill)
            .padding(iced::Padding {
                top: 0.0,
                right: 8.0,
                bottom: 8.0,
                left: 8.0,
            }),
        chrome_line(),
        container(add_plugin_button()).padding(iced::Padding {
            top: 10.0,
            right: 12.0,
            bottom: 10.0,
            left: 12.0,
        }),
    ];

    container(column)
        .width(Length::Fixed(272.0))
        .height(Length::Fill)
        .style(|_| container::Style {
            background: Some(MASTER_BG.into()),
            ..Default::default()
        })
        .into()
}

fn master_row(plugin: &PmPlugin, active: bool) -> Element<'_, PrismEvent> {
    let content = row![
        letter_tile(plugin.letter, 30.0, 7.0, plugin.color, plugin.ink),
        column![
            text(plugin.name.clone())
                .size(14.0)
                .font(typo::TITLE_M.2)
                .color(colors::ON_SURFACE),
            text(command_meta(plugin))
                .size(11.0)
                .color(colors::SECONDARY),
        ]
        .spacing(1.0)
        .width(Length::Fill),
        pill_toggle(
            plugin.enabled,
            34.0,
            20.0,
            PrismEvent::PluginManager(PmEvent::ToggleEnabled(plugin.id.clone())),
        ),
    ]
    .spacing(11.0)
    .align_y(Alignment::Center);

    button(content)
        .on_press(PrismEvent::PluginManager(PmEvent::Select(
            plugin.id.clone(),
        )))
        .width(Length::Fill)
        .padding(spacing::SPACE_S)
        .style(move |_, status| {
            let hovered = status == button::Status::Hovered;
            button::Style {
                background: (active || hovered).then(|| colors::ON_SURFACE.scale_alpha(0.1).into()),
                text_color: colors::ON_SURFACE,
                border: iced::Border {
                    radius: 8.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
        .into()
}

fn command_meta(plugin: &PmPlugin) -> String {
    let count = plugin.commands.len();
    let noun = if count == 1 { "command" } else { "commands" };
    format!("{count} {noun} · v{}", plugin.version)
}

fn add_plugin_button<'a>() -> Element<'a, PrismEvent> {
    // Presentational (matches the source design's non-wired button).
    container(
        row![
            text("＋").size(15.0).color(colors::ON_SURFACE),
            text("Add Plugin")
                .size(13.0)
                .font(typo::TITLE_M.2)
                .color(colors::ON_SURFACE),
        ]
        .spacing(spacing::SPACE_S)
        .align_y(Alignment::Center),
    )
    .center_x(Length::Fill)
    .padding(spacing::SPACE_S)
    .style(|_| container::Style {
        background: Some(colors::ON_SURFACE.scale_alpha(0.06).into()),
        border: iced::Border {
            color: colors::ON_SURFACE.scale_alpha(0.1),
            width: 1.0,
            radius: 8.0.into(),
        },
        ..Default::default()
    })
    .into()
}

// --- Detail (selected plugin) -----------------------------------------------

fn detail(state: &PluginManagerState) -> Element<'_, PrismEvent> {
    let inner: Element<PrismEvent> = match state.selected() {
        Some(plugin) => detail_content(plugin),
        None => empty_detail(),
    };

    container(
        scrollable(container(inner).padding(iced::Padding {
            top: 24.0,
            right: 28.0,
            bottom: 24.0,
            left: 28.0,
        }))
        .height(Length::Fill)
        .direction(slim_scrollbar())
        .style(scrollbar_style),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .style(|_| container::Style {
        background: Some(WINDOW_BG.into()),
        ..Default::default()
    })
    .into()
}

fn detail_content(plugin: &PmPlugin) -> Element<'_, PrismEvent> {
    let header = row![
        letter_tile(plugin.letter, 56.0, 14.0, plugin.color, plugin.ink),
        column![
            text(plugin.name.clone())
                .size(22.0)
                .font(typo::TITLE_M.2)
                .color(colors::ON_SURFACE),
            text(format!("by {} · v{}", plugin.author, plugin.version))
                .size(13.0)
                .color(colors::ON_SURFACE_VARIANT),
        ]
        .spacing(3.0)
        .width(Length::Fill),
        status_badge(plugin.enabled),
    ]
    .spacing(spacing::SPACE_M)
    .align_y(Alignment::Start);

    let mut content = column![
        header,
        container(
            text(plugin.desc.clone())
                .size(14.0)
                .line_height(iced::widget::text::LineHeight::Relative(1.55))
                .color(DESC_FG)
                .width(Length::Fixed(520.0)),
        )
        .padding(iced::Padding {
            top: 14.0,
            right: 0.0,
            bottom: 0.0,
            left: 0.0,
        }),
    ]
    .width(Length::Fill);

    if !plugin.prefs.is_empty() {
        content = content
            .push(section_label("Preferences", 26.0))
            .push(prefs_card(plugin));
    }

    if !plugin.commands.is_empty() {
        content = content
            .push(section_label("Commands", 24.0))
            .push(commands_list(plugin));
    }

    content = content
        .push(vgap(24.0))
        .push(meta_cells(plugin))
        .push(footer_actions());

    content.into()
}

fn empty_detail<'a>() -> Element<'a, PrismEvent> {
    container(
        column![
            text("No plugin selected")
                .size(18.0)
                .font(typo::TITLE_M.2)
                .color(colors::ON_SURFACE),
            text("Every plugin has been removed. Add one to get started.")
                .size(13.0)
                .color(colors::ON_SURFACE_VARIANT),
        ]
        .spacing(spacing::SPACE_S)
        .align_x(Alignment::Center),
    )
    .center(Length::Fill)
    .height(Length::Fixed(360.0))
    .into()
}

fn status_badge<'a>(enabled: bool) -> Element<'a, PrismEvent> {
    let (label, dot, fg, bg) = if enabled {
        (
            "Enabled",
            colors::TERTIARY,
            colors::TERTIARY,
            colors::TERTIARY.scale_alpha(0.12),
        )
    } else {
        (
            "Disabled",
            colors::SECONDARY,
            colors::SECONDARY,
            colors::ON_SURFACE.scale_alpha(0.06),
        )
    };

    container(
        row![
            container(text(""))
                .width(Length::Fixed(7.0))
                .height(Length::Fixed(7.0))
                .style(move |_| container::Style {
                    background: Some(dot.into()),
                    border: iced::Border {
                        radius: 4.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
            text(label).size(12.0).font(typo::LABEL_M.2).color(fg),
        ]
        .spacing(6.0)
        .align_y(Alignment::Center),
    )
    .padding(iced::Padding {
        top: 5.0,
        right: 10.0,
        bottom: 5.0,
        left: 10.0,
    })
    .style(move |_| container::Style {
        background: Some(bg.into()),
        border: iced::Border {
            radius: 7.0.into(),
            ..Default::default()
        },
        ..Default::default()
    })
    .into()
}

fn prefs_card(plugin: &PmPlugin) -> Element<'_, PrismEvent> {
    let mut rows = column![];
    let last = plugin.prefs.len().saturating_sub(1);

    for (index, pref) in plugin.prefs.iter().enumerate() {
        let control: Element<PrismEvent> = match &pref.control {
            PmControl::Toggle(on) => pill_toggle(
                *on,
                40.0,
                24.0,
                PrismEvent::PluginManager(PmEvent::ActivatePref {
                    plugin: plugin.id.clone(),
                    index,
                }),
            ),
            PmControl::Select { options, selected } => {
                select_dropdown(options, *selected, plugin.id.clone(), index)
            }
            PmControl::Text(value) => pref_text_input(value, false, plugin.id.clone(), index),
            PmControl::Secret(value) => pref_text_input(value, true, plugin.id.clone(), index),
        };

        let pref_row = row![
            column![
                text(pref.label.clone())
                    .size(14.0)
                    .font(typo::TITLE_M.2)
                    .color(colors::ON_SURFACE),
                text(pref.hint.clone()).size(12.0).color(colors::SECONDARY),
            ]
            .spacing(2.0)
            .width(Length::Fill),
            control,
        ]
        .spacing(spacing::SPACE_M)
        .align_y(Alignment::Center);

        rows = rows.push(container(pref_row).padding(iced::Padding {
            top: 14.0,
            right: 16.0,
            bottom: 14.0,
            left: 16.0,
        }));
        if index != last {
            rows = rows.push(hline());
        }
    }

    section_top(12.0, card(rows.into()))
}

/// An editable preference field for a text or secret value. Secrets are masked.
/// Each keystroke emits `EditPref` (which persists and notifies the plugin).
fn pref_text_input<'a>(
    value: &str,
    secret: bool,
    plugin_id: String,
    index: usize,
) -> Element<'a, PrismEvent> {
    let placeholder = if secret { "Enter a value…" } else { "" };
    let mut input = text_input(placeholder, value)
        .on_input(move |value| {
            PrismEvent::PluginManager(PmEvent::EditPref {
                plugin: plugin_id.clone(),
                index,
                value,
            })
        })
        .size(13.0)
        .font(if secret {
            typo::CODE_M.2
        } else {
            typo::BODY_M.2
        })
        .width(Length::Fixed(190.0))
        .padding(iced::Padding {
            top: 7.0,
            right: 12.0,
            bottom: 7.0,
            left: 12.0,
        })
        .style(|_theme, _status| iced::widget::text_input::Style {
            background: colors::ON_SURFACE.scale_alpha(0.05).into(),
            border: iced::Border {
                color: colors::ON_SURFACE.scale_alpha(0.14),
                width: 1.0,
                radius: 8.0.into(),
            },
            icon: Color::WHITE,
            placeholder: colors::SECONDARY,
            value: colors::ON_SURFACE,
            selection: colors::ON_SURFACE.scale_alpha(0.3),
        });
    if secret {
        input = input.secure(true);
    }
    input.into()
}

/// A real dropdown for a select preference: shows the current option and, when
/// clicked, opens a menu of all options. Choosing one emits `SelectPref`.
fn select_dropdown<'a>(
    options: &[String],
    selected: usize,
    plugin_id: String,
    index: usize,
) -> Element<'a, PrismEvent> {
    let opts: Vec<String> = options.to_vec();
    let lookup = opts.clone();
    let current = opts.get(selected).cloned();

    pick_list(opts, current, move |choice: String| {
        let option = lookup.iter().position(|o| *o == choice).unwrap_or(0);
        PrismEvent::PluginManager(PmEvent::SelectPref {
            plugin: plugin_id.clone(),
            index,
            option,
        })
    })
    .text_size(13.0)
    .font(typo::BODY_M.2)
    .padding(iced::Padding {
        top: 7.0,
        right: 10.0,
        bottom: 7.0,
        left: 12.0,
    })
    .style(|_theme, status| {
        let active = matches!(
            status,
            pick_list::Status::Hovered | pick_list::Status::Opened { .. }
        );
        pick_list::Style {
            text_color: colors::ON_SURFACE,
            placeholder_color: colors::SECONDARY,
            handle_color: colors::SECONDARY,
            background: colors::ON_SURFACE
                .scale_alpha(if active { 0.1 } else { 0.05 })
                .into(),
            border: iced::Border {
                color: colors::ON_SURFACE.scale_alpha(0.14),
                width: 1.0,
                radius: 8.0.into(),
            },
        }
    })
    .menu_style(|_theme| iced::overlay::menu::Style {
        background: CARD_BG.into(),
        border: iced::Border {
            color: colors::ON_SURFACE.scale_alpha(0.14),
            width: 1.0,
            radius: 8.0.into(),
        },
        text_color: colors::ON_SURFACE,
        selected_text_color: colors::ON_SURFACE,
        selected_background: colors::ON_SURFACE.scale_alpha(0.1).into(),
        shadow: iced::Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.4),
            offset: iced::Vector::new(0.0, 8.0),
            blur_radius: 24.0,
        },
    })
    .into()
}

fn commands_list(plugin: &PmPlugin) -> Element<'_, PrismEvent> {
    let mut list = column![].spacing(spacing::SPACE_S);

    for command in &plugin.commands {
        let trailing: Element<PrismEvent> = match &command.hotkey {
            Some(hotkey) => hotkey_chip(hotkey),
            None => record_hotkey_chip(),
        };

        let command_row = row![
            letter_tile(command.glyph, 26.0, 7.0, command.color, command.ink),
            column![
                text(command.name.clone())
                    .size(14.0)
                    .font(typo::TITLE_M.2)
                    .color(colors::ON_SURFACE),
                text(command.desc.clone())
                    .size(12.0)
                    .color(colors::SECONDARY),
            ]
            .spacing(1.0)
            .width(Length::Fill),
            trailing,
        ]
        .spacing(spacing::SPACE_S + spacing::SPACE_XS)
        .align_y(Alignment::Center);

        list = list.push(
            container(command_row)
                .padding(iced::Padding {
                    top: 11.0,
                    right: 14.0,
                    bottom: 11.0,
                    left: 14.0,
                })
                .style(|_| container::Style {
                    background: Some(colors::ON_SURFACE.scale_alpha(0.03).into()),
                    border: iced::Border {
                        color: colors::ON_SURFACE.scale_alpha(0.07),
                        width: 1.0,
                        radius: 9.0.into(),
                    },
                    ..Default::default()
                }),
        );
    }

    section_top(12.0, list.into())
}

fn hotkey_chip<'a>(hotkey: &str) -> Element<'a, PrismEvent> {
    container(
        text(hotkey.to_string())
            .size(12.0)
            .font(typo::CODE_S.2)
            .color(colors::ON_SURFACE_VARIANT),
    )
    .padding(iced::Padding {
        top: 3.0,
        right: 9.0,
        bottom: 3.0,
        left: 9.0,
    })
    .style(|_| container::Style {
        background: Some(colors::ON_SURFACE.scale_alpha(0.06).into()),
        border: iced::Border {
            color: colors::ON_SURFACE.scale_alpha(0.12),
            width: 1.0,
            radius: 6.0.into(),
        },
        ..Default::default()
    })
    .into()
}

fn record_hotkey_chip<'a>() -> Element<'a, PrismEvent> {
    container(text("Record Hotkey").size(12.0).color(colors::SECONDARY))
        .padding(iced::Padding {
            top: 3.0,
            right: 10.0,
            bottom: 3.0,
            left: 10.0,
        })
        .style(|_| container::Style {
            border: iced::Border {
                color: colors::ON_SURFACE.scale_alpha(0.2),
                width: 1.0,
                radius: 6.0.into(),
            },
            ..Default::default()
        })
        .into()
}

fn meta_cells(plugin: &PmPlugin) -> Element<'_, PrismEvent> {
    let cells = [
        ("Version", plugin.version.clone()),
        ("Updated", plugin.updated.clone()),
        ("Size", plugin.size.clone()),
    ];

    let mut row = row![];
    let last = cells.len() - 1;
    for (index, (key, value)) in cells.into_iter().enumerate() {
        row = row.push(
            container(
                column![
                    text(key.to_string())
                        .size(11.0)
                        .font(typo::LABEL_S.2)
                        .color(colors::SECONDARY),
                    text(value).size(13.0).color(colors::ON_SURFACE),
                ]
                .spacing(3.0),
            )
            .width(Length::Fill)
            .padding(iced::Padding {
                top: 12.0,
                right: 16.0,
                bottom: 12.0,
                left: 16.0,
            }),
        );
        if index != last {
            row = row.push(vline(Length::Fixed(52.0)));
        }
    }

    section_top(24.0, card(row.into()))
}

fn footer_actions<'a>() -> Element<'a, PrismEvent> {
    // "Check for Updates" is presentational; "Uninstall" opens the confirm flow.
    let check = container(
        row![
            text("↻").size(14.0).color(colors::ON_SURFACE),
            text("Check for Updates")
                .size(13.0)
                .font(typo::TITLE_M.2)
                .color(colors::ON_SURFACE),
        ]
        .spacing(spacing::SPACE_S)
        .align_y(Alignment::Center),
    )
    .padding(iced::Padding {
        top: 9.0,
        right: 14.0,
        bottom: 9.0,
        left: 14.0,
    })
    .style(|_| container::Style {
        background: Some(colors::ON_SURFACE.scale_alpha(0.06).into()),
        border: iced::Border {
            color: colors::ON_SURFACE.scale_alpha(0.1),
            width: 1.0,
            radius: 8.0.into(),
        },
        ..Default::default()
    });

    let uninstall = button(
        row![
            text("🗑").size(14.0).color(DANGER_FG),
            text("Uninstall")
                .size(13.0)
                .font(typo::TITLE_M.2)
                .color(DANGER_FG),
        ]
        .spacing(spacing::SPACE_S)
        .align_y(Alignment::Center),
    )
    .on_press(PrismEvent::PluginManager(PmEvent::UninstallRequest))
    .padding(iced::Padding {
        top: 9.0,
        right: 16.0,
        bottom: 9.0,
        left: 16.0,
    })
    .style(|_, status| {
        let hovered = status == button::Status::Hovered;
        button::Style {
            background: Some(
                colors::PRIMARY
                    .scale_alpha(if hovered { 0.2 } else { 0.12 })
                    .into(),
            ),
            text_color: DANGER_FG,
            border: iced::Border {
                color: colors::PRIMARY.scale_alpha(0.35),
                width: 1.0,
                radius: 8.0.into(),
            },
            ..Default::default()
        }
    });

    section_top(
        20.0,
        row![check, horizontal(), uninstall]
            .spacing(spacing::SPACE_M)
            .align_y(Alignment::Center)
            .into(),
    )
}

// --- Uninstall confirmation -------------------------------------------------

fn uninstall_overlay(plugin: &PmPlugin) -> Element<'_, PrismEvent> {
    let command_count = plugin.commands.len();
    let noun = if command_count == 1 {
        "command"
    } else {
        "commands"
    };

    let card = column![
        container(text("🗑").size(24.0))
            .center(Length::Fixed(52.0))
            .style(|_| container::Style {
                background: Some(colors::PRIMARY.scale_alpha(0.14).into()),
                border: iced::Border {
                    color: colors::PRIMARY.scale_alpha(0.3),
                    width: 1.0,
                    radius: 13.0.into(),
                },
                ..Default::default()
            }),
        text(format!("Uninstall {}?", plugin.name))
            .size(17.0)
            .font(typo::TITLE_M.2)
            .color(colors::ON_SURFACE),
        text(format!(
            "This removes the plugin and its {command_count} {noun}. Your preferences for it will be deleted."
        ))
        .size(13.0)
        .line_height(iced::widget::text::LineHeight::Relative(1.5))
        .color(colors::ON_SURFACE_VARIANT)
        .align_x(Alignment::Center)
        .width(Length::Fixed(336.0)),
        row![
            confirm_button("Cancel", false, PrismEvent::PluginManager(PmEvent::UninstallCancel)),
            confirm_button("Uninstall", true, PrismEvent::PluginManager(PmEvent::UninstallConfirm)),
        ]
        .spacing(10.0)
        .width(Length::Fill),
    ]
    .spacing(spacing::SPACE_M)
    .align_x(Alignment::Center);

    let dialog = container(card)
        .width(Length::Fixed(380.0))
        .padding(22.0)
        .style(|_| container::Style {
            background: Some(CARD_BG.into()),
            border: iced::Border {
                color: colors::ON_SURFACE.scale_alpha(0.14),
                width: 1.0,
                radius: 14.0.into(),
            },
            ..Default::default()
        });

    container(dialog)
        .width(Length::Fill)
        .height(Length::Fill)
        .center(Length::Fill)
        .style(|_| container::Style {
            background: Some(Color::from_rgba(0.02, 0.03, 0.035, 0.55).into()),
            ..Default::default()
        })
        .into()
}

fn confirm_button<'a>(label: &str, danger: bool, message: PrismEvent) -> Element<'a, PrismEvent> {
    button(
        container(text(label.to_string()).size(13.0).font(typo::TITLE_M.2)).center_x(Length::Fill),
    )
    .on_press(message)
    .width(Length::Fill)
    .padding(10.0)
    .style(move |_, status| {
        let hovered = status == button::Status::Hovered;
        if danger {
            button::Style {
                background: Some(
                    if hovered {
                        colors::PRIMARY.scale_alpha(0.85)
                    } else {
                        colors::PRIMARY
                    }
                    .into(),
                ),
                text_color: Color::WHITE,
                border: iced::Border {
                    radius: 8.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        } else {
            button::Style {
                background: Some(
                    colors::ON_SURFACE
                        .scale_alpha(if hovered { 0.1 } else { 0.06 })
                        .into(),
                ),
                text_color: colors::ON_SURFACE,
                border: iced::Border {
                    color: colors::ON_SURFACE.scale_alpha(0.12),
                    width: 1.0,
                    radius: 8.0.into(),
                },
                ..Default::default()
            }
        }
    })
    .into()
}

// --- Shared building blocks -------------------------------------------------

/// A colored square with a centered glyph — the icon tile used throughout.
fn letter_tile<'a>(
    glyph: char,
    size: f32,
    radius: f32,
    bg: Color,
    ink: Color,
) -> Element<'a, PrismEvent> {
    container(
        text(glyph.to_string())
            .size(size * 0.46)
            .font(typo::TITLE_M.2)
            .color(ink),
    )
    .center(Length::Fixed(size))
    .style(move |_| container::Style {
        background: Some(bg.into()),
        border: iced::Border {
            radius: radius.into(),
            ..Default::default()
        },
        ..Default::default()
    })
    .into()
}

/// A rounded pill switch with a white knob; teal when on, gray when off.
fn pill_toggle<'a>(
    on: bool,
    width: f32,
    height: f32,
    message: PrismEvent,
) -> Element<'a, PrismEvent> {
    let knob = container(text(""))
        .width(Length::Fixed(height - 4.0))
        .height(Length::Fixed(height - 4.0))
        .style(move |_| container::Style {
            background: Some(Color::WHITE.into()),
            border: iced::Border {
                radius: ((height - 4.0) / 2.0).into(),
                ..Default::default()
            },
            ..Default::default()
        });

    let inner = if on {
        row![horizontal(), knob]
    } else {
        row![knob, horizontal()]
    }
    .height(Length::Fill)
    .align_y(Alignment::Center);

    let track_bg = if on {
        colors::TERTIARY
    } else {
        colors::ON_SURFACE.scale_alpha(0.16)
    };

    let pill = container(inner)
        .width(Length::Fixed(width))
        .height(Length::Fixed(height))
        .padding(2.0)
        .style(move |_| container::Style {
            background: Some(track_bg.into()),
            border: iced::Border {
                radius: (height / 2.0).into(),
                ..Default::default()
            },
            ..Default::default()
        });

    button(pill)
        .padding(0.0)
        .on_press(message)
        .style(|_, _| button::Style {
            background: None,
            ..Default::default()
        })
        .into()
}

/// A rounded card container for grouped rows.
fn card<'a>(content: Element<'a, PrismEvent>) -> Element<'a, PrismEvent> {
    container(content)
        .width(Length::Fill)
        .style(|_| container::Style {
            background: Some(colors::ON_SURFACE.scale_alpha(0.03).into()),
            border: iced::Border {
                color: colors::ON_SURFACE.scale_alpha(0.07),
                width: 1.0,
                radius: 10.0.into(),
            },
            ..Default::default()
        })
        .clip(true)
        .into()
}

/// An uppercase section label, preceded by `top` padding.
fn section_label<'a>(label: &str, top: f32) -> Element<'a, PrismEvent> {
    section_top(
        top,
        text(label.to_uppercase())
            .size(12.0)
            .font(typo::LABEL_M.2)
            .color(colors::SECONDARY)
            .into(),
    )
}

/// Add `top` padding above `content` (vertical rhythm between sections).
fn section_top<'a>(top: f32, content: Element<'a, PrismEvent>) -> Element<'a, PrismEvent> {
    container(content)
        .width(Length::Fill)
        .padding(iced::Padding {
            top,
            right: 0.0,
            bottom: 0.0,
            left: 0.0,
        })
        .into()
}

/// A transparent spacer that adds vertical rhythm between sections.
fn vgap<'a>(height: f32) -> Element<'a, PrismEvent> {
    container(text("")).height(Length::Fixed(height)).into()
}

/// A full-width 1px interior divider.
fn hline<'a>() -> Element<'a, PrismEvent> {
    container(text(""))
        .width(Length::Fill)
        .height(Length::Fixed(1.0))
        .style(|_| container::Style {
            background: Some(soft_line().into()),
            ..Default::default()
        })
        .into()
}

/// A 1px vertical divider of the given height.
fn vline<'a>(height: Length) -> Element<'a, PrismEvent> {
    container(text(""))
        .width(Length::Fixed(1.0))
        .height(height)
        .style(|_| container::Style {
            background: Some(soft_line().into()),
            ..Default::default()
        })
        .into()
}

/// A 1px horizontal chrome divider (darker than interior dividers).
fn chrome_line<'a>() -> Element<'a, PrismEvent> {
    container(text(""))
        .width(Length::Fill)
        .height(Length::Fixed(1.0))
        .style(|_| container::Style {
            background: Some(HAIRLINE.into()),
            ..Default::default()
        })
        .into()
}
