use crate::app::Raycast;

mod app;
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
        Raycast::default,
        Raycast::namespace,
        Raycast::update,
        Raycast::view,
    )
    .style(Raycast::style)
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
