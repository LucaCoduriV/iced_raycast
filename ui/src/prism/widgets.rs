use crate::design_system::icons;
use core::Image;
use iced::{
    Alignment, Background, Color, Element, Length, gradient,
    widget::{Id, Row, container, image, svg, text, text_input},
    widget::{button, column, row, space::horizontal},
};

use crate::design_system::typo::Typography;
use crate::{
    design_system::{colors, spacing, typo},
    prism::items::{IconHandle, ListEntry},
};

/// A specialized search input with transparent styling
pub fn search_bar<'a, Message>(
    id: Id,
    query: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
    argument_id: Id,
    argument: Option<&'a str>,
    on_argument_input: impl Fn(String) -> Message + 'a,
    icon: Option<Image>,
    show_argument_input: bool,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
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
    let kind: &str = entry.kind();

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

pub fn render_icon<'a, Message>(icon_handler: IconHandle, size: f32) -> Element<'a, Message> {
    match icon_handler {
        IconHandle::Svg(handle) => svg(handle)
            .width(Length::Fixed(size))
            .height(Length::Fixed(size))
            .into(),
        IconHandle::Other(handle) => image(handle)
            .width(Length::Fixed(size))
            .height(Length::Fixed(size))
            .into(),
    }
}
