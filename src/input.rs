#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Feed,
    AddFish,
    Shark,
    Quit,
}

/// Pure key→action mapping (unit-testable without a terminal).
pub fn action_for_key(c: char) -> Option<Action> {
    match c.to_ascii_lowercase() {
        'f' => Some(Action::Feed),
        'a' => Some(Action::AddFish),
        's' => Some(Action::Shark),
        'q' => Some(Action::Quit),
        _ => None,
    }
}

use crossterm::event::{self, Event, KeyCode};
use std::time::Duration;

/// Poll for a key without blocking. Returns the mapped action, if any.
/// `Resize` events are surfaced separately by the caller via `poll_event`.
pub fn poll_action(timeout: Duration) -> std::io::Result<Option<Action>> {
    if event::poll(timeout)? {
        if let Event::Key(k) = event::read()? {
            if let KeyCode::Char(c) = k.code {
                return Ok(action_for_key(c));
            }
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_known_keys() {
        assert_eq!(action_for_key('f'), Some(Action::Feed));
        assert_eq!(action_for_key('a'), Some(Action::AddFish));
        assert_eq!(action_for_key('s'), Some(Action::Shark));
        assert_eq!(action_for_key('q'), Some(Action::Quit));
    }

    #[test]
    fn unknown_keys_map_to_none() {
        assert_eq!(action_for_key('z'), None);
    }

    #[test]
    fn is_case_insensitive() {
        assert_eq!(action_for_key('Q'), Some(Action::Quit));
    }
}
