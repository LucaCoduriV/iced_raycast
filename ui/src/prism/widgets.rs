use crate::design_system::icons;
use core::Image;
use iced::{
    Alignment, Background, Color, Element, Length, gradient,
    widget::{Id, Row, container, image, scrollable, svg, text, text_input},
    widget::{button, column, row, space::horizontal},
};

use crate::design_system::typo::Typography;
use crate::{
    design_system::{colors, spacing, typo},
    prism::items::{IconHandle, ListEntry},
};

/// Inputs for [`search_bar`]. Grouped into a struct so call sites read as
/// named fields rather than a long positional argument list.
pub struct SearchBar<'a, Message> {
    pub id: Id,
    pub query: &'a str,
    pub on_input: Box<dyn Fn(String) -> Message + 'a>,
    pub argument_id: Id,
    pub argument: Option<&'a str>,
    pub on_argument_input: Box<dyn Fn(String) -> Message + 'a>,
    pub icon: Option<Image>,
    pub show_argument_input: bool,
}

/// A specialized search input with transparent styling
pub fn search_bar<'a, Message>(props: SearchBar<'a, Message>) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let SearchBar {
        id,
        query,
        on_input,
        argument_id,
        argument,
        on_argument_input,
        icon,
        show_argument_input,
    } = props;

    let search_input = text_input("Search for apps and commands...", query)
        .on_input(on_input)
        .id(id)
        .size(typo::TITLE_L.0)
        .font(typo::TITLE_L.2)
        .padding(15)
        .width(Length::FillPortion(1))
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

    let mut row = Row::new().push(search_input);

    if show_argument_input {
        if let Some(icon) = icon {
            let icon_handle: IconHandle = icon.into();
            row = row.push(render_icon(icon_handle, icons::MD));
        }

        let argument_input = text_input("Argument...", argument.unwrap_or_default())
            .on_input(on_argument_input)
            .id(argument_id)
            .size(typo::TITLE_L.0)
            .font(typo::TITLE_L.2)
            .padding(15)
            .width(Length::FillPortion(1))
            .align_x(Alignment::End)
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

        row = row.push(argument_input);
    }

    row.into()
}

/// Inputs for [`argument_view`].
pub struct ArgumentView<'a, Message> {
    pub command_name: &'a str,
    pub icon: IconHandle,
    pub description: Option<&'a str>,
    pub argument_id: Id,
    pub argument: Option<&'a str>,
    pub on_input: Box<dyn Fn(String) -> Message + 'a>,
    pub recent: &'a [String],
    pub on_recent: Box<dyn Fn(String) -> Message + 'a>,
}

/// The argument-entry screen: a command token pill + argument input, with a
/// list of recently-used arguments for that command.
pub fn argument_view<'a, Message: 'a + Clone>(
    props: ArgumentView<'a, Message>,
) -> Element<'a, Message> {
    let ArgumentView {
        command_name,
        icon,
        description,
        argument_id,
        argument,
        on_input,
        recent,
        on_recent,
    } = props;

    let pill = container(
        row![
            render_icon(icon, 22.0),
            text(command_name.to_string())
                .size(15.0)
                .font(typo::TITLE_M.2)
                .color(colors::ON_SURFACE),
            kbd("esc"),
        ]
        .spacing(spacing::SPACE_S)
        .align_y(Alignment::Center),
    )
    .padding(iced::Padding {
        top: 6.0,
        right: 6.0,
        bottom: 6.0,
        left: 8.0,
    })
    .style(|_| container::Style {
        background: Some(colors::TERTIARY.scale_alpha(0.14).into()),
        border: iced::Border {
            color: colors::TERTIARY.scale_alpha(0.32),
            width: 1.0,
            radius: 10.0.into(),
        },
        ..Default::default()
    });

    let input = text_input("Argument...", argument.unwrap_or_default())
        .on_input(on_input)
        .id(argument_id)
        .size(typo::TITLE_L.0)
        .font(typo::TITLE_L.2)
        .style(|_theme, _status| text_input::Style {
            background: Color::TRANSPARENT.into(),
            border: iced::Border {
                width: 0.0,
                ..Default::default()
            },
            icon: Color::WHITE,
            placeholder: colors::ON_SURFACE_VARIANT,
            value: Color::WHITE,
            selection: colors::ON_SURFACE.scale_alpha(0.3),
        });

    let header = container(
        row![pill, text("›").size(20.0).color(colors::SECONDARY), input]
            .spacing(spacing::SPACE_S + spacing::SPACE_XS)
            .align_y(Alignment::Center),
    )
    .padding(12.0);

    let mut body = column![].spacing(spacing::SPACE_XS).width(Length::Fill);

    if let Some(desc) = description {
        body = body.push(
            container(
                text(format!(
                    "Runs {command_name} with the argument you type. {desc}"
                ))
                .size(13.0)
                .color(colors::ON_SURFACE_VARIANT),
            )
            .padding(iced::Padding {
                top: 10.0,
                right: 8.0,
                bottom: 14.0,
                left: 8.0,
            }),
        );
    }

    if !recent.is_empty() {
        body = body.push(section_header("Recent arguments"));
        for value in recent {
            let selected_value = value.clone();
            let content = row![
                container(text("↺").size(14.0).color(colors::SECONDARY))
                    .center(Length::Fixed(28.0))
                    .style(|_| container::Style {
                        background: Some(colors::ON_SURFACE.scale_alpha(0.08).into()),
                        border: iced::Border {
                            color: colors::ON_SURFACE.scale_alpha(0.1),
                            width: 1.0,
                            radius: 7.0.into(),
                        },
                        ..Default::default()
                    }),
                text(value.clone())
                    .font(typo::CODE_M.2)
                    .size(14.0)
                    .color(colors::ON_SURFACE),
            ]
            .spacing(spacing::SPACE_M)
            .align_y(Alignment::Center);

            body = body.push(
                button(content)
                    .on_press(on_recent(selected_value))
                    .width(Length::Fill)
                    .padding(spacing::SPACE_S)
                    .style(|_theme, status| {
                        let hovered = status == button::Status::Hovered;
                        let bg = if hovered {
                            colors::ON_SURFACE.scale_alpha(0.1)
                        } else {
                            Color::TRANSPARENT
                        };

                        button::Style {
                            background: Some(bg.into()),
                            text_color: colors::ON_SURFACE,
                            border: iced::Border {
                                radius: 8.0.into(),
                                ..Default::default()
                            },
                            ..Default::default()
                        }
                    }),
            );
        }
    }

    column![
        header,
        divider(),
        scrollable(container(body).padding(spacing::SPACE_S))
            .width(Length::Fill)
            .height(Length::Fill)
            .direction(slim_scrollbar())
            .style(scrollbar_style),
    ]
    .height(Length::Fill)
    .into()
}

/// A slim, translucent scrollbar matching the launcher's surface treatment:
/// no visible track, a rounded thumb that brightens on hover/drag.
pub fn scrollbar_style(theme: &iced::Theme, status: scrollable::Status) -> scrollable::Style {
    let active = matches!(
        status,
        scrollable::Status::Hovered {
            is_vertical_scrollbar_hovered: true,
            ..
        } | scrollable::Status::Dragged {
            is_vertical_scrollbar_dragged: true,
            ..
        }
    );

    let rail = scrollable::Rail {
        background: None,
        border: iced::Border::default(),
        scroller: scrollable::Scroller {
            background: colors::ON_SURFACE
                .scale_alpha(if active { 0.30 } else { 0.15 })
                .into(),
            border: iced::Border {
                radius: 4.0.into(),
                ..Default::default()
            },
        },
    };

    scrollable::Style {
        vertical_rail: rail,
        horizontal_rail: rail,
        ..scrollable::default(theme, status)
    }
}

/// Slim vertical scrollbar geometry to pair with [`scrollbar_style`].
pub fn slim_scrollbar() -> scrollable::Direction {
    scrollable::Direction::Vertical(
        scrollable::Scrollbar::default()
            .width(8.0)
            .scroller_width(6.0)
            .margin(2.0),
    )
}

/// An uppercase section label that heads a group of results.
pub fn section_header<'a, Message: 'a>(label: &str) -> Element<'a, Message> {
    container(
        text(label.to_uppercase())
            .typography(typo::LABEL_M)
            .color(colors::SECONDARY),
    )
    .padding(iced::Padding {
        top: spacing::SPACE_S,
        right: spacing::SPACE_S,
        bottom: spacing::SPACE_XS,
        left: spacing::SPACE_S,
    })
    .into()
}

/// A gradient divider line
pub fn divider<'a, Message: 'a>() -> Element<'a, Message> {
    container("")
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
        })
        .into()
}

/// A clickable list entry with selection state styling
pub fn list_item<'a, Message>(
    entry: &'a ListEntry,
    is_selected: bool,
    on_press: Message,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let kind: &str = entry.kind_label();

    let content = row![
        render_icon(entry.icon(), icons::LG),
        column![
            text(entry.name())
                .typography(typo::TITLE_M)
                .color(colors::ON_SURFACE),
            text(entry.description().unwrap_or(""))
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
    .align_y(Alignment::Center);

    button(content)
        .on_press(on_press)
        .width(Length::Fill)
        .padding(spacing::SPACE_S)
        .style(move |_theme, status| {
            let is_hovered = status == button::Status::Hovered;

            let bg_color = if is_selected || is_hovered {
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
                ..Default::default()
            }
        })
        .into()
}

/// What invoking a menu action does.
#[derive(Debug, Clone)]
pub enum MenuActionKind {
    /// Same as pressing Enter on the item (open app / run command).
    Primary,
    /// Copy the item's name to the clipboard.
    CopyName,
    /// Perform a plugin-provided effect (copy / push view / close).
    Effect(core::ActionEffect),
}

/// A single row in the actions menu.
#[derive(Debug, Clone)]
pub struct MenuAction {
    pub label: String,
    pub hint: &'static str,
    pub glyph: &'static str,
    pub color: Color,
    pub kind: MenuActionKind,
}

/// The contextual actions available for the selected entry.
pub fn actions_for(entry: &ListEntry) -> Vec<MenuAction> {
    // Plugin results carry their own actions (e.g. "Copy to Clipboard").
    let plugin_actions = entry.entity.plugin_actions();
    if !plugin_actions.is_empty() {
        return plugin_actions
            .iter()
            .enumerate()
            .map(|(i, action)| MenuAction {
                label: action.label.clone(),
                hint: if i == 0 { "↵" } else { "" },
                glyph: "⧉",
                color: if i == 0 {
                    colors::PRIMARY
                } else {
                    colors::SECONDARY
                },
                kind: MenuActionKind::Effect(action.effect.clone()),
            })
            .collect();
    }

    let is_app = entry.kind_label() == "Application";

    vec![
        MenuAction {
            label: if is_app {
                "Open".into()
            } else {
                "Run Command".into()
            },
            hint: "↵",
            glyph: "↵",
            color: colors::PRIMARY,
            kind: MenuActionKind::Primary,
        },
        MenuAction {
            label: "Copy Name".into(),
            hint: "⌘C",
            glyph: "⧉",
            color: colors::SECONDARY,
            kind: MenuActionKind::CopyName,
        },
    ]
}

/// The actions popover (⌘K). Rendered as an overlay anchored above the footer.
pub fn actions_menu<'a, Message: 'a + Clone>(
    actions: Vec<MenuAction>,
    selected: usize,
    on_select: impl Fn(usize) -> Message + 'a,
) -> Element<'a, Message> {
    let mut items = column![
        container(
            text("Actions")
                .font(typo::LABEL_S.2)
                .size(11.0)
                .color(colors::SECONDARY)
        )
        .padding(iced::Padding {
            top: 4.0,
            right: 6.0,
            bottom: 8.0,
            left: 6.0,
        })
    ]
    .width(Length::Fill);

    for (i, action) in actions.into_iter().enumerate() {
        let is_selected = i == selected;
        let color = action.color;

        let glyph = container(
            text(action.glyph.to_string())
                .size(12.0)
                .color(Color::WHITE),
        )
        .center(Length::Fixed(22.0))
        .style(move |_| container::Style {
            background: Some(color.into()),
            border: iced::Border {
                radius: 6.0.into(),
                ..Default::default()
            },
            ..Default::default()
        });

        let content = row![
            glyph,
            text(action.label).size(14.0).color(colors::ON_SURFACE),
            horizontal(),
            kbd(action.hint),
        ]
        .spacing(spacing::SPACE_S + spacing::SPACE_XS)
        .align_y(Alignment::Center);

        items = items.push(
            button(content)
                .on_press(on_select(i))
                .width(Length::Fill)
                .padding(spacing::SPACE_S)
                .style(move |_theme, status| {
                    let hovered = status == button::Status::Hovered;
                    let bg = if is_selected || hovered {
                        colors::ON_SURFACE.scale_alpha(0.1)
                    } else {
                        Color::TRANSPARENT
                    };

                    button::Style {
                        background: Some(bg.into()),
                        text_color: colors::ON_SURFACE,
                        border: iced::Border {
                            radius: 8.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                }),
        );
    }

    container(items)
        .width(Length::Fixed(280.0))
        .padding(spacing::SPACE_S)
        .style(|_| container::Style {
            background: Some(Color::from_rgba8(30, 30, 33, 0.96).into()),
            border: iced::Border {
                color: colors::ON_SURFACE.scale_alpha(0.14),
                width: 1.0,
                radius: 12.0.into(),
            },
            ..Default::default()
        })
        .into()
}

/// Centered "no results" state shown when the filtered list is empty.
pub fn empty_state<'a, Message: 'a>(query: &str) -> Element<'a, Message> {
    let icon_box = container(text("⌕").size(30.0).color(colors::SECONDARY))
        .center(Length::Fixed(64.0))
        .style(|_| container::Style {
            background: Some(colors::ON_SURFACE.scale_alpha(0.06).into()),
            border: iced::Border {
                color: colors::ON_SURFACE.scale_alpha(0.1),
                width: 1.0,
                radius: 16.0.into(),
            },
            ..Default::default()
        });

    let title = if query.is_empty() {
        "No results".to_string()
    } else {
        format!("No results for “{query}”")
    };

    container(
        column![
            icon_box,
            text(title)
                .size(18.0)
                .font(typo::TITLE_M.2)
                .color(colors::ON_SURFACE),
            text("Nothing matched your search. Try a different name, or check your spelling.")
                .size(14.0)
                .color(colors::ON_SURFACE_VARIANT)
                .width(Length::Fixed(340.0))
                .align_x(Alignment::Center),
        ]
        .spacing(spacing::SPACE_M)
        .align_x(Alignment::Center),
    )
    .center(Length::Fill)
    .into()
}

/// A keyboard-hint chip: a monospace glyph in a subtle rounded box.
pub fn kbd<'a, Message: 'a>(label: &str) -> Element<'a, Message> {
    container(
        text(label.to_string())
            .font(typo::CODE_S.2)
            .size(12.0)
            .color(colors::ON_SURFACE_VARIANT),
    )
    .padding(iced::Padding {
        top: 2.0,
        right: 7.0,
        bottom: 2.0,
        left: 7.0,
    })
    .style(|_| container::Style {
        background: Some(colors::ON_SURFACE.scale_alpha(0.08).into()),
        border: iced::Border {
            color: colors::ON_SURFACE.scale_alpha(0.12),
            width: 1.0,
            radius: 5.0.into(),
        },
        ..Default::default()
    })
    .into()
}

/// Thin vertical rule separating footer action groups.
fn footer_separator<'a, Message: 'a>() -> Element<'a, Message> {
    container("")
        .width(Length::Fixed(1.0))
        .height(Length::Fixed(18.0))
        .style(|_| container::Style {
            background: Some(colors::ON_SURFACE.scale_alpha(0.12).into()),
            ..Default::default()
        })
        .into()
}

/// Shared chrome for a bottom bar: a hairline top border and a fixed-height,
/// horizontally-padded row.
fn footer_shell<'a, Message: 'a>(bar: Row<'a, Message>) -> Element<'a, Message> {
    column![
        container("")
            .width(Length::Fill)
            .height(Length::Fixed(1.0))
            .style(|_| container::Style {
                background: Some(colors::ON_SURFACE.scale_alpha(0.08).into()),
                ..Default::default()
            }),
        container(bar)
            .width(Length::Fill)
            .center_y(Length::Fixed(42.0))
            .padding(iced::Padding {
                top: 0.0,
                right: 12.0,
                bottom: 0.0,
                left: 12.0,
            }),
    ]
    .into()
}

/// The bottom action bar. Shows the selected item's tile and its primary
/// action, plus the Actions (⌘K) affordance; falls back to a no-selection
/// state when nothing is selected.
pub fn footer<'a, Message: 'a>(selected: Option<&'a ListEntry>) -> Element<'a, Message> {
    let bar = match selected {
        Some(entry) => {
            let primary = entry.primary_action_label();

            row![
                render_icon(entry.icon(), icons::SM),
                horizontal(),
                text(primary).size(13.0).color(colors::ON_SURFACE),
                kbd("↵"),
                footer_separator(),
                text("Actions").size(13.0).color(colors::ON_SURFACE),
                kbd("⌘K"),
            ]
        }
        None => row![
            text("No selection").size(13.0).color(colors::SECONDARY),
            horizontal(),
            text("Clear search").size(13.0).color(colors::ON_SURFACE),
            kbd("esc"),
        ],
    }
    .spacing(spacing::SPACE_S)
    .align_y(Alignment::Center);

    footer_shell(bar)
}

/// The bottom bar shown while entering a command argument: Run / Cancel.
pub fn argument_footer<'a, Message: 'a>(
    entry: &'a ListEntry,
    argument: Option<&str>,
) -> Element<'a, Message> {
    let run_label = match argument {
        Some(arg) if !arg.is_empty() => format!("Run “{arg}”"),
        _ => "Run".to_string(),
    };

    let bar = row![
        render_icon(entry.icon(), icons::SM),
        horizontal(),
        text(run_label).size(13.0).color(colors::ON_SURFACE),
        kbd("↵"),
        footer_separator(),
        text("Cancel").size(13.0).color(colors::ON_SURFACE),
        kbd("esc"),
    ]
    .spacing(spacing::SPACE_S)
    .align_y(Alignment::Center);

    footer_shell(bar)
}

pub fn render_icon<'a, Message: 'a>(icon_handler: IconHandle, size: f32) -> Element<'a, Message> {
    match icon_handler {
        IconHandle::Svg(handle) => svg(handle)
            .width(Length::Fixed(size))
            .height(Length::Fixed(size))
            .into(),
        IconHandle::Other(handle) => image(handle)
            .width(Length::Fixed(size))
            .height(Length::Fixed(size))
            .into(),
        IconHandle::Letter { letter, color } => container(
            text(letter.to_string())
                .size(size * 0.46)
                .font(typo::TITLE_M.2)
                .color(Color::WHITE),
        )
        .center(Length::Fixed(size))
        .style(move |_| container::Style {
            background: Some(color.into()),
            border: iced::Border {
                radius: (size * 0.25).into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into(),
    }
}
