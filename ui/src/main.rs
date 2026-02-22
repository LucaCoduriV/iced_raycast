#![allow(clippy::too_many_arguments)]
use crate::app::Raycast;

mod app;
mod design_system;
mod prism;

#[cfg(not(target_os = "linux"))]
pub fn main() -> iced::Result {
    use iced::{Size, advanced::graphics::core::window};

    iced::application(Raycast::new, Raycast::update, Raycast::view)
        .style(Raycast::style)
        .font(include_bytes!("../fonts/Roboto-Regular.ttf").as_slice())
        .font(include_bytes!("../fonts/Roboto-Medium.ttf").as_slice())
        .font(include_bytes!("../fonts/RobotoMono-Regular.ttf").as_slice())
        .subscription(Raycast::subscription)
        .window(window::Settings {
            size: Size {
                width: 700.,
                height: 500.,
            },
            position: window::Position::Centered,
            resizable: false,
            closeable: false,
            minimizable: false,
            decorations: false,
            transparent: true,
            blur: true,
            level: window::Level::AlwaysOnTop,
            ..window::Settings::default()
        })
        .run()
}

#[cfg(target_os = "linux")]
pub fn main() -> Result<(), iced_layershell::Error> {
    use iced_layershell::application;
    use iced_layershell::reexport::{Anchor, KeyboardInteractivity};
    use iced_layershell::settings::{LayerShellSettings, Settings};

    application(
        Raycast::new,
        Raycast::namespace,
        Raycast::update,
        Raycast::view,
    )
    .style(Raycast::style)
    .font(include_bytes!("../fonts/Roboto-Regular.ttf").as_slice())
    .font(include_bytes!("../fonts/Roboto-Medium.ttf").as_slice())
    .font(include_bytes!("../fonts/RobotoMono-Regular.ttf").as_slice())
    .subscription(Raycast::subscription)
    .settings(Settings {
        layer_settings: LayerShellSettings {
            size: Some((700, 500)),
            exclusive_zone: -1,
            anchor: Anchor::empty(),
            keyboard_interactivity: KeyboardInteractivity::OnDemand,
            ..Default::default()
        },
        ..Default::default()
    })
    .run()
}
