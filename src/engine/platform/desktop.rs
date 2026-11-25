use super::{GameState, InputQueue};
use crate::engine::input::Key;
use crate::engine::terminal::Ctx;
use minifb::{Key as MKey, KeyRepeat, MouseButton, MouseMode, Scale, ScaleMode, Window, WindowOptions};
use std::time::Instant;

pub fn run<G: GameState>(mut ctx: Ctx, mut gs: G) -> super::BError {
    let (w, h) = (ctx.screen().width, ctx.screen().height);
    let mut window = Window::new(&ctx.title, w, h, WindowOptions {
        resize: true,
        scale: Scale::X1,
        scale_mode: ScaleMode::AspectRatioStretch,
        ..WindowOptions::default()
    })?;
    window.set_target_fps(60);

    let mut input = InputQueue::default();
    let mut last = Instant::now();
    let mut shots = 0;
    while window.is_open() && !ctx.quitting {
        for k in window.get_keys_pressed(KeyRepeat::Yes) {
            if k == MKey::F12 {
                shots += 1;
                let path = format!("screenshot-{shots}.png");
                match std::fs::write(&path, ctx.screen().to_png()) {
                    Ok(()) => super::log(format!("saved {path}")),
                    Err(e) => super::log(format!("couldn't save {path}: {e}")),
                }
            } else if let Some(key) = map_key(k) {
                input.keys.push_back(key);
            }
        }
        ctx.shift = window.is_key_down(MKey::LeftShift) || window.is_key_down(MKey::RightShift);
        ctx.control = window.is_key_down(MKey::LeftCtrl) || window.is_key_down(MKey::RightCtrl);
        ctx.alt = window.is_key_down(MKey::LeftAlt) || window.is_key_down(MKey::RightAlt);
        input.mouse_down = window.get_mouse_down(MouseButton::Left);

        if let Some((mx, my)) = window.get_unscaled_mouse_pos(MouseMode::Pass) {
            ctx.mouse_px = window_to_buffer(window.get_size(), (w, h), (mx, my));
        }
        input.apply(&mut ctx);

        let now = Instant::now();
        ctx.frame_time_ms = (now - last).as_secs_f32() * 1000.0;
        last = now;

        gs.tick(&mut ctx);
        let screen = ctx.render();
        window.update_with_buffer(&screen.pixels, screen.width, screen.height)?;
    }
    Ok(())
}

fn window_to_buffer((ww, wh): (usize, usize), (bw, bh): (usize, usize), (mx, my): (f32, f32)) -> (i32, i32) {
    let scale = (ww as f32 / bw as f32).min(wh as f32 / bh as f32).max(1e-3);
    let ox = (ww as f32 - bw as f32 * scale) / 2.0;
    let oy = (wh as f32 - bh as f32 * scale) / 2.0;
    (((mx - ox) / scale) as i32, ((my - oy) / scale) as i32)
}

fn map_key(k: MKey) -> Option<Key> {
    use MKey as M;
    Some(match k {
        M::A => Key::A, M::B => Key::B, M::C => Key::C, M::D => Key::D, M::E => Key::E, M::F => Key::F,
        M::G => Key::G, M::H => Key::H, M::I => Key::I, M::J => Key::J, M::K => Key::K, M::L => Key::L,
        M::M => Key::M, M::N => Key::N, M::O => Key::O, M::P => Key::P, M::Q => Key::Q, M::R => Key::R,
        M::S => Key::S, M::T => Key::T, M::U => Key::U, M::V => Key::V, M::W => Key::W, M::X => Key::X,
        M::Y => Key::Y, M::Z => Key::Z,
        M::Key0 => Key::Key0, M::Key1 => Key::Key1, M::Key2 => Key::Key2, M::Key3 => Key::Key3,
        M::Key4 => Key::Key4, M::Key5 => Key::Key5, M::Key6 => Key::Key6, M::Key7 => Key::Key7,
        M::Key8 => Key::Key8, M::Key9 => Key::Key9,
        M::NumPad0 => Key::Numpad0, M::NumPad1 => Key::Numpad1, M::NumPad2 => Key::Numpad2,
        M::NumPad3 => Key::Numpad3, M::NumPad4 => Key::Numpad4, M::NumPad5 => Key::Numpad5,
        M::NumPad6 => Key::Numpad6, M::NumPad7 => Key::Numpad7, M::NumPad8 => Key::Numpad8,
        M::NumPad9 => Key::Numpad9, M::NumPadPlus => Key::NumpadAdd, M::NumPadMinus => Key::NumpadSubtract,
        M::NumPadAsterisk => Key::NumpadMultiply, M::NumPadSlash => Key::NumpadDivide,
        M::NumPadDot => Key::NumpadDecimal, M::NumPadEnter => Key::NumpadEnter,
        M::F1 => Key::F1, M::F2 => Key::F2, M::F3 => Key::F3, M::F4 => Key::F4, M::F5 => Key::F5,
        M::F6 => Key::F6, M::F7 => Key::F7, M::F8 => Key::F8, M::F9 => Key::F9, M::F10 => Key::F10,
        M::F11 => Key::F11,
        M::Up => Key::Up, M::Down => Key::Down, M::Left => Key::Left, M::Right => Key::Right,
        M::Enter => Key::Return, M::Escape => Key::Escape, M::Space => Key::Space, M::Tab => Key::Tab,
        M::Backspace => Key::Back, M::Insert => Key::Insert, M::Delete => Key::Delete, M::Home => Key::Home,
        M::End => Key::End, M::PageUp => Key::PageUp, M::PageDown => Key::PageDown,
        M::Comma => Key::Comma, M::Period => Key::Period, M::Slash => Key::Slash,
        M::Backslash => Key::Backslash, M::Semicolon => Key::Semicolon, M::Apostrophe => Key::Apostrophe,
        M::Minus => Key::Minus, M::Equal => Key::Equals, M::LeftBracket => Key::LBracket,
        M::RightBracket => Key::RBracket, M::Backquote => Key::Grave,
        _ => return None,
    })
}
 
#[cfg(test)]
mod tests {
    use super::window_to_buffer;
 
    #[test]
    fn mouse_maps_through_letterbox() {
        // 1280x960 buffer in a 1920x1080 window: scaled 1.125 to 1440x1080, 240px bars each side.
        assert_eq!(window_to_buffer((1920, 1080), (1280, 960), (240.0, 0.0)), (0, 0));
        assert_eq!(window_to_buffer((1920, 1080), (1280, 960), (1680.0, 1080.0)), (1280, 960));
        assert_eq!(window_to_buffer((1280, 960), (1280, 960), (640.0, 480.0)), (640, 480));
    }
}