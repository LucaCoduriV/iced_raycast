use iced::keyboard;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Key {
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Enter,
    Escape,
    Other,
}

impl From<keyboard::Key> for Key {
    fn from(key: keyboard::Key) -> Self {
        match key {
            keyboard::Key::Named(keyboard::key::Named::ArrowUp) => Key::ArrowUp,
            keyboard::Key::Named(keyboard::key::Named::ArrowDown) => Key::ArrowDown,
            keyboard::Key::Named(keyboard::key::Named::ArrowLeft) => Key::ArrowLeft,
            keyboard::Key::Named(keyboard::key::Named::ArrowRight) => Key::ArrowRight,
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
    SelectLeft,
    SelectRight,
    Submit,
    EscapePressed,
    ToggleActions,
}

pub fn map_key_to_action(key: &keyboard::Key, modifiers: keyboard::Modifiers) -> Option<KeyAction> {
    // ⌘K / Ctrl+K toggles the actions menu. Other modified chords are ignored
    // so they don't get misread as list navigation.
    if modifiers.command() {
        if let keyboard::Key::Character(c) = key
            && c.as_str().eq_ignore_ascii_case("k")
        {
            return Some(KeyAction::ToggleActions);
        }
        return None;
    }

    match Key::from(key.clone()) {
        Key::ArrowUp => Some(KeyAction::SelectPrevious),
        Key::ArrowDown => Some(KeyAction::SelectNext),
        Key::ArrowLeft => Some(KeyAction::SelectLeft),
        Key::ArrowRight => Some(KeyAction::SelectRight),
        Key::Enter => Some(KeyAction::Submit),
        Key::Escape => Some(KeyAction::EscapePressed),
        _ => None,
    }
}
