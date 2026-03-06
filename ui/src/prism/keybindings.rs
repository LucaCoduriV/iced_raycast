use iced::keyboard;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Key {
    ArrowUp,
    ArrowDown,
    Enter,
    Escape,
    Other,
}

impl From<keyboard::Key> for Key {
    fn from(key: keyboard::Key) -> Self {
        match key {
            keyboard::Key::Named(keyboard::key::Named::ArrowUp) => Key::ArrowUp,
            keyboard::Key::Named(keyboard::key::Named::ArrowDown) => Key::ArrowDown,
            keyboard::Key::Named(keyboard::key::Named::Enter) => Key::Enter,
            keyboard::Key::Named(keyboard::key::Named::Escape) => Key::Escape,
            _ => Key::Other,
        }
    }
}

#[derive(Debug, Clone)]
pub enum KeyAction {
    SelectPrevious,
    SelectNext,
    Submit,
    EscapePressed,
}

pub fn map_key_to_action(key: keyboard::Key) -> Option<KeyAction> {
    match Key::from(key) {
        Key::ArrowUp => Some(KeyAction::SelectPrevious),
        Key::ArrowDown => Some(KeyAction::SelectNext),
        Key::Enter => Some(KeyAction::Submit),
        Key::Escape => Some(KeyAction::EscapePressed),
        _ => None,
    }
}
