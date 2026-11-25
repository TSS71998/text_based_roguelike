use crate::engine::{Direction, Key};
use crate::render::ViewMode;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Move(Direction),
    Step(i32),
    Turn(i32),
    Wait,
    PickUp,
    Inventory,
    Drop,
    Remove,
    Descend,
    Ascend,
    UseHotkey(i32),
    SaveAndQuit,
    CheatMenu,
    ToggleView,
}

pub fn action_for_key(key: Key, view: ViewMode) -> Option<Action> {
    if let Some(n) = hotkey_number(key) {
        return Some(Action::UseHotkey(n));
    }
    let movement = match view {
        ViewMode::TopDown => match key {
            Key::Left | Key::A => Some(Action::Move(Direction::West)),
            Key::Right | Key::D => Some(Action::Move(Direction::East)),
            Key::Up | Key::W => Some(Action::Move(Direction::North)),
            Key::Down | Key::S => Some(Action::Move(Direction::South)),
            Key::Q => Some(Action::Move(Direction::NorthWest)),
            Key::E => Some(Action::Move(Direction::NorthEast)),
            Key::Z => Some(Action::Move(Direction::SouthWest)),
            Key::C => Some(Action::Move(Direction::SouthEast)),
            _ => None,
        },
        ViewMode::FirstPerson => match key {
            Key::Up | Key::W => Some(Action::Step(0)),
            Key::Down | Key::S => Some(Action::Step(4)),
            Key::Q => Some(Action::Step(6)),
            Key::E => Some(Action::Step(2)),
            Key::Left | Key::A => Some(Action::Turn(-2)),
            Key::Right | Key::D => Some(Action::Turn(2)),
            _ => None,
        },
    };
    movement.or(match key {
        Key::G => Some(Action::PickUp),
        Key::I => Some(Action::Inventory),
        Key::B => Some(Action::Drop),
        Key::R => Some(Action::Remove),
        Key::Period => Some(Action::Descend),
        Key::Comma => Some(Action::Ascend),
        Key::Space => Some(Action::Wait),
        Key::Escape => Some(Action::SaveAndQuit),
        Key::Backslash => Some(Action::CheatMenu),
        Key::Tab => Some(Action::ToggleView),
        _ => None,
    })
}
 
/// Keys 1-9 use consumables 0-8.
fn hotkey_number(key: Key) -> Option<i32> {
    const KEYS: [Key; 9] = [Key::Key1, Key::Key2, Key::Key3, Key::Key4, Key::Key5, Key::Key6, Key::Key7, Key::Key8, Key::Key9];
    KEYS.iter().position(|&k| k == key).map(|i| i as i32)
}
 
#[cfg(test)]
mod tests {
    use super::*;
 
    #[test]
    fn views_share_actions_but_not_movement() {
        assert_eq!(action_for_key(Key::Up, ViewMode::TopDown), Some(Action::Move(Direction::North)));
        assert_eq!(action_for_key(Key::Up, ViewMode::FirstPerson), Some(Action::Step(0)));
        assert_eq!(action_for_key(Key::Left, ViewMode::FirstPerson), Some(Action::Turn(-2)));
        assert_eq!(action_for_key(Key::E, ViewMode::FirstPerson), Some(Action::Step(2)));
        for view in [ViewMode::TopDown, ViewMode::FirstPerson] {
            assert_eq!(action_for_key(Key::I, view), Some(Action::Inventory));
            assert_eq!(action_for_key(Key::Key3, view), Some(Action::UseHotkey(2)));
            assert_eq!(action_for_key(Key::Tab, view), Some(Action::ToggleView));
            assert_eq!(action_for_key(Key::F5, view), None);
        }
    }
 
    #[test]
    fn steps_resolve_against_facing() {
        // Facing east, a "step right" (2 eighths clockwise) goes south.
        assert_eq!(Direction::East.rotated(2), Direction::South);
        assert_eq!(Direction::East.rotated(4), Direction::West);
        assert_eq!(Direction::East.rotated(6), Direction::North);
    }
}