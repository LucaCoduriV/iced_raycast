use crate::app::Raycast;

mod app;
mod design_system;
mod prism;

#[cfg(not(target_os = "linux"))]
pub fn main() -> Result<(), Error> {
    iced::run(Raycast::update, Raycast::view)
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
