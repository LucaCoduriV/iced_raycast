//! Renderers for full-screen plugin views (grid / detail / form).

use core::{FieldKind, FieldValueKind, GridItem, ImageSource, ListRow, ViewBody};
use iced::{
    Alignment, Color, ContentFit, Element, Length,
    widget::{
        button, column, container, image, pick_list, row, scrollable, space::horizontal, text,
        text_input, toggler,
    },
};

use super::PrismEvent;
use super::state::{ImageCache, ImageEntry, ViewState};
use super::widgets::{kbd, scrollbar_style, slim_scrollbar};
use crate::design_system::typo::Typography;
use crate::design_system::{colors, spacing, typo};

/// Fixed height of a grid cell's title (≈ two lines of `LABEL_M`), clipped so
/// cells stay uniform regardless of title length.
const GRID_TITLE_HEIGHT: f32 = 32.0;
/// Fixed height of a grid cell's optional subtitle (one line of `BODY_S`).
const GRID_SUBTITLE_HEIGHT: f32 = 16.0;

/// Render the active plugin view: header, optional search, body, footer.
pub fn view_screen<'a>(
    state: &'a ViewState,
    images: &'a ImageCache,
    elapsed_ms: u64,
) -> Element<'a, PrismEvent> {
    let header = row![
        back_button(),
        text(state.view.title.as_str())
            .typography(typo::TITLE_M)
            .color(colors::ON_SURFACE),
        horizontal(),
    ]
    .spacing(spacing::SPACE_S)
    .align_y(Alignment::Center);

    let mut screen = column![container(header).padding(iced::Padding {
        top: 4.0,
        right: 8.0,
        bottom: 8.0,
        left: 4.0,
    })];

    if let Some(placeholder) = &state.view.search_placeholder {
        screen = screen.push(search_bar(state, placeholder));
    }

    screen = screen.push(super::widgets::divider());

    let body: Element<PrismEvent> = match &state.view.body {
        ViewBody::Grid { columns, items } => grid_body(
            *columns,
            items,
            state.selected,
            images,
            elapsed_ms,
            &state.row_ids,
        ),
        ViewBody::List { items } => list_body(items, state.selected),
        ViewBody::Detail { body, metadata } => detail_body(body, metadata),
        ViewBody::Form { .. } => form_body(state),
    };

    screen = screen.push(
        scrollable(container(body).padding(spacing::SPACE_S))
            .id(state.scroll_id.clone())
            .on_scroll(PrismEvent::ViewScrolled)
            .height(Length::Fill)
            .width(Length::Fill)
            .direction(slim_scrollbar())
            .style(scrollbar_style),
    );

    screen = screen.push(footer(state));

    screen.height(Length::Fill).into()
}

fn back_button() -> Element<'static, PrismEvent> {
    button(text("‹").size(20.0).color(colors::ON_SURFACE))
        .on_press(PrismEvent::PopView)
        .padding(iced::Padding {
            top: 0.0,
            right: 8.0,
            bottom: 2.0,
            left: 8.0,
        })
        .style(|_theme, status| {
            let hovered = status == button::Status::Hovered;
            button::Style {
                background: Some(
                    colors::ON_SURFACE
                        .scale_alpha(if hovered { 0.14 } else { 0.08 })
                        .into(),
                ),
                text_color: colors::ON_SURFACE,
                border: iced::Border {
                    radius: 7.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
        .into()
}

fn search_bar<'a>(state: &'a ViewState, placeholder: &'a str) -> Element<'a, PrismEvent> {
    text_input(placeholder, &state.search)
        .id(state.search_id.clone())
        .on_input(PrismEvent::ViewSearchInput)
        .size(typo::TITLE_M.0)
        .font(typo::TITLE_M.2)
        .padding(iced::Padding {
            top: 8.0,
            right: 8.0,
            bottom: 8.0,
            left: 8.0,
        })
        .style(transparent_input_style)
        .into()
}

fn transparent_input_style(_theme: &iced::Theme, _status: text_input::Status) -> text_input::Style {
    text_input::Style {
        background: Color::TRANSPARENT.into(),
        border: iced::Border {
            width: 0.0,
            ..Default::default()
        },
        icon: Color::WHITE,
        placeholder: colors::ON_SURFACE_VARIANT,
        value: Color::WHITE,
        selection: colors::ON_SURFACE.scale_alpha(0.3),
    }
}

// --- Grid -------------------------------------------------------------------

fn grid_body<'a>(
    columns: u32,
    items: &'a [GridItem],
    selected: usize,
    images: &'a ImageCache,
    elapsed_ms: u64,
    row_ids: &'a [iced::widget::Id],
) -> Element<'a, PrismEvent> {
    let columns = columns.max(1) as usize;
    let mut grid = column![].spacing(spacing::SPACE_M);

    for (row_index, chunk) in items.chunks(columns).enumerate() {
        let mut cells = row![].spacing(spacing::SPACE_M);
        for (col_index, item) in chunk.iter().enumerate() {
            let index = row_index * columns + col_index;
            cells = cells.push(grid_cell(item, index == selected, images, elapsed_ms));
        }
        // Pad the final row so cells keep their column width.
        for _ in chunk.len()..columns {
            cells = cells.push(horizontal());
        }

        // Tag the row with its id so the host can measure its height.
        let row: Element<PrismEvent> = match row_ids.get(row_index) {
            Some(id) => container(cells).id(id.clone()).into(),
            None => cells.into(),
        };
        grid = grid.push(row);
    }

    grid.into()
}

fn grid_cell<'a>(
    item: &'a GridItem,
    is_selected: bool,
    images: &'a ImageCache,
    elapsed_ms: u64,
) -> Element<'a, PrismEvent> {
    let subtitle = item.subtitle.as_deref().unwrap_or("");

    // Clamp the title to a fixed two-line box (clipped) so every cell — and
    // therefore every grid row — is the same height, regardless of how long
    // the title is.
    let title = container(
        text(item.title.as_str())
            .typography(typo::LABEL_M)
            .color(colors::ON_SURFACE)
            .align_x(iced::alignment::Horizontal::Center)
            .width(Length::Fill),
    )
    .height(Length::Fixed(GRID_TITLE_HEIGHT))
    .width(Length::Fill)
    .clip(true);

    let mut content = column![grid_image(&item.image, images, elapsed_ms), title]
        .spacing(spacing::SPACE_XS)
        .width(Length::Fill);

    if !subtitle.is_empty() {
        content = content.push(
            container(
                text(subtitle)
                    .typography(typo::BODY_S)
                    .color(colors::ON_SURFACE_VARIANT)
                    .align_x(iced::alignment::Horizontal::Center)
                    .width(Length::Fill),
            )
            .height(Length::Fixed(GRID_SUBTITLE_HEIGHT))
            .width(Length::Fill)
            .clip(true),
        );
    }

    button(content)
        .on_press(PrismEvent::ViewItemActivated(item.id.clone()))
        .width(Length::Fill)
        .padding(spacing::SPACE_XS)
        .style(move |_theme, status| {
            let hovered = status == button::Status::Hovered;
            button::Style {
                background: (is_selected || hovered)
                    .then(|| colors::ON_SURFACE.scale_alpha(0.08).into()),
                text_color: colors::ON_SURFACE,
                border: iced::Border {
                    color: if is_selected {
                        colors::TERTIARY
                    } else {
                        Color::TRANSPARENT
                    },
                    width: if is_selected { 1.0 } else { 0.0 },
                    radius: 10.0.into(),
                },
                ..Default::default()
            }
        })
        .into()
}

fn image_tile(element: Element<'_, PrismEvent>) -> Element<'_, PrismEvent> {
    container(element)
        .center(Length::Fill)
        .width(Length::Fill)
        .height(Length::Fixed(96.0))
        .style(|_| container::Style {
            background: Some(colors::ON_SURFACE.scale_alpha(0.06).into()),
            border: iced::Border {
                radius: 8.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}

fn cover(handle: image::Handle) -> Element<'static, PrismEvent> {
    image_tile(
        image(handle)
            .width(Length::Fill)
            .height(Length::Fill)
            .content_fit(ContentFit::Cover)
            .into(),
    )
}

fn placeholder(label: &str) -> Element<'static, PrismEvent> {
    image_tile(
        text(label.to_string())
            .size(13.0)
            .color(colors::SECONDARY)
            .into(),
    )
}

fn grid_image<'a>(
    source: &'a ImageSource,
    images: &'a ImageCache,
    elapsed_ms: u64,
) -> Element<'a, PrismEvent> {
    match source {
        // Remote images are fetched, decoded, and cached by the host; render
        // the current animation frame (stable handle id, no reload) or a
        // placeholder while it loads.
        ImageSource::Url(url) => match images.get(url) {
            Some(entry @ ImageEntry::Loaded(_)) => match entry.frame_at(elapsed_ms) {
                Some(handle) => cover(handle.clone()),
                None => placeholder("image"),
            },
            Some(ImageEntry::Failed) => placeholder("✕"),
            _ => placeholder("…"),
        },
        ImageSource::Path(path) => cover(image::Handle::from_path(path)),
        ImageSource::Bytes(bytes) => cover(image::Handle::from_bytes(bytes.clone())),
        ImageSource::None => placeholder("image"),
    }
}

// --- List -------------------------------------------------------------------

fn list_body(items: &[ListRow], selected: usize) -> Element<'_, PrismEvent> {
    if items.is_empty() {
        return container(
            text("Nothing here yet. Copy something, then reopen this list.")
                .size(13.0)
                .color(colors::ON_SURFACE_VARIANT),
        )
        .center_x(Length::Fill)
        .height(Length::Fixed(200.0))
        .into();
    }

    let mut list = column![].spacing(spacing::SPACE_XS).width(Length::Fill);
    for (index, item) in items.iter().enumerate() {
        list = list.push(list_row(item, index == selected));
    }
    list.into()
}

fn list_row(item: &ListRow, is_selected: bool) -> Element<'_, PrismEvent> {
    let glyph = item.glyph.unwrap_or('•');
    let tile = container(
        text(glyph.to_string())
            .size(15.0)
            .font(typo::TITLE_M.2)
            .color(colors::ON_SURFACE),
    )
    .center(Length::Fixed(30.0))
    .style(|_| container::Style {
        background: Some(colors::ON_SURFACE.scale_alpha(0.06).into()),
        border: iced::Border {
            radius: 7.0.into(),
            ..Default::default()
        },
        ..Default::default()
    });

    let mut lines = column![
        text(item.title.clone())
            .typography(typo::TITLE_S)
            .color(colors::ON_SURFACE)
    ]
    .spacing(1.0)
    .width(Length::Fill);
    if let Some(subtitle) = &item.subtitle {
        lines = lines.push(
            text(subtitle.clone())
                .typography(typo::BODY_S)
                .color(colors::ON_SURFACE_VARIANT),
        );
    }

    let content = row![tile, lines]
        .spacing(spacing::SPACE_M)
        .align_y(Alignment::Center);

    button(content)
        .on_press(PrismEvent::ViewItemActivated(item.id.clone()))
        .width(Length::Fill)
        .padding(spacing::SPACE_S)
        .style(move |_theme, status| {
            let hovered = status == button::Status::Hovered;
            button::Style {
                background: (is_selected || hovered)
                    .then(|| colors::ON_SURFACE.scale_alpha(0.1).into()),
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

// --- Detail -----------------------------------------------------------------

fn detail_body<'a>(body: &'a str, metadata: &'a [core::KeyValue]) -> Element<'a, PrismEvent> {
    let mut text_column = column![].spacing(spacing::SPACE_S).width(Length::Fill);
    for paragraph in body.split("\n\n") {
        text_column = text_column.push(
            text(paragraph.trim())
                .typography(typo::BODY_M)
                .color(colors::ON_SURFACE),
        );
    }

    let mut sidebar = column![]
        .spacing(spacing::SPACE_M)
        .width(Length::Fixed(200.0));
    for entry in metadata {
        sidebar = sidebar.push(
            column![
                text(entry.key.to_uppercase())
                    .typography(typo::LABEL_S)
                    .color(colors::SECONDARY),
                text(entry.value.as_str())
                    .typography(typo::BODY_M)
                    .color(colors::ON_SURFACE),
            ]
            .spacing(spacing::SPACE_XXS),
        );
    }

    row![text_column, sidebar].spacing(spacing::SPACE_L).into()
}

// --- Form -------------------------------------------------------------------

fn form_body(state: &ViewState) -> Element<'_, PrismEvent> {
    let ViewBody::Form { fields } = &state.view.body else {
        return column![].into();
    };

    let mut form = column![].spacing(spacing::SPACE_M).width(Length::Fill);

    for field in fields {
        let control: Element<PrismEvent> = match &field.kind {
            FieldKind::Text(initial) | FieldKind::TextArea(initial) => {
                let value = text_value(state, &field.id, initial);
                let field_id = field.id.clone();
                let mono = matches!(field.kind, FieldKind::TextArea(_));
                let mut input = text_input("", value)
                    .on_input(move |v| PrismEvent::ViewFormText {
                        field_id: field_id.clone(),
                        value: v,
                    })
                    .padding(spacing::SPACE_S)
                    .style(form_input_style);
                if mono {
                    input = input.font(typo::CODE_M.2);
                }
                input.into()
            }
            FieldKind::Toggle(initial) => {
                let on = toggle_value(state, &field.id, *initial);
                let field_id = field.id.clone();
                toggler(on)
                    .on_toggle(move |v| PrismEvent::ViewFormToggle {
                        field_id: field_id.clone(),
                        value: v,
                    })
                    .into()
            }
            FieldKind::Dropdown { options, selected } => {
                let current = choice_value(state, &field.id, *selected) as usize;
                let options = options.clone();
                let field_id = field.id.clone();
                let selected_label = options.get(current).cloned();
                let lookup = options.clone();
                pick_list(options, selected_label, move |choice: String| {
                    let index = lookup.iter().position(|o| *o == choice).unwrap_or(0) as u64;
                    PrismEvent::ViewFormChoice {
                        field_id: field_id.clone(),
                        index,
                    }
                })
                .into()
            }
        };

        form = form.push(
            column![
                text(field.label.as_str())
                    .typography(typo::LABEL_M)
                    .color(colors::ON_SURFACE_VARIANT),
                control,
            ]
            .spacing(spacing::SPACE_XS),
        );
    }

    form.into()
}

fn form_input_style(_theme: &iced::Theme, _status: text_input::Status) -> text_input::Style {
    text_input::Style {
        background: colors::ON_SURFACE.scale_alpha(0.05).into(),
        border: iced::Border {
            color: colors::ON_SURFACE.scale_alpha(0.12),
            width: 1.0,
            radius: 8.0.into(),
        },
        icon: Color::WHITE,
        placeholder: colors::ON_SURFACE_VARIANT,
        value: Color::WHITE,
        selection: colors::ON_SURFACE.scale_alpha(0.3),
    }
}

fn text_value<'a>(state: &'a ViewState, id: &str, initial: &'a str) -> &'a str {
    match state.form_values.get(id) {
        Some(FieldValueKind::Text(value)) => value,
        _ => initial,
    }
}

fn toggle_value(state: &ViewState, id: &str, initial: bool) -> bool {
    match state.form_values.get(id) {
        Some(FieldValueKind::Toggle(value)) => *value,
        _ => initial,
    }
}

fn choice_value(state: &ViewState, id: &str, initial: u64) -> u64 {
    match state.form_values.get(id) {
        Some(FieldValueKind::Choice(value)) => *value,
        _ => initial,
    }
}

// --- Footer -----------------------------------------------------------------

fn footer(state: &ViewState) -> Element<'_, PrismEvent> {
    let submit = state.view.submit_label.as_deref().unwrap_or("Select");

    let mut bar = row![].spacing(spacing::SPACE_S).align_y(Alignment::Center);

    // Loading feedback while a search request is in flight.
    if state.searching {
        bar = bar.push(
            text("Searching…")
                .size(13.0)
                .color(colors::ON_SURFACE_VARIANT),
        );
    }

    bar = bar.push(horizontal());
    bar = bar.push(text(submit).size(13.0).color(colors::TERTIARY));
    bar = bar.push(kbd("↵"));
    bar = bar.push(
        container("")
            .width(Length::Fixed(1.0))
            .height(Length::Fixed(18.0))
            .style(|_| container::Style {
                background: Some(colors::ON_SURFACE.scale_alpha(0.12).into()),
                ..Default::default()
            }),
    );
    bar = bar.push(text("Back").size(13.0).color(colors::ON_SURFACE));
    bar = bar.push(kbd("esc"));

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
