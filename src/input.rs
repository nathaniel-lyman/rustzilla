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

/// A single thing the loop reacts to: a mapped key action or a terminal resize.
pub enum Input {
    Action(Action),
    Resize(u16, u16),
}

/// Poll for input without blocking. Returns a mapped key action, a resize, or
/// `None` if nothing happened within `timeout`.
pub fn poll_input(timeout: Duration) -> std::io::Result<Option<Input>> {
    if event::poll(timeout)? {
        match event::read()? {
            Event::Key(k) => {
                if let KeyCode::Char(c) = k.code {
                    return Ok(action_for_key(c).map(Input::Action));
                }
            }
            Event::Resize(w, h) => return Ok(Some(Input::Resize(w, h))),
            _ => {}
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
