use super::{GameState, InputQueue};
use crate::engine::input::Key;
use crate::engine::terminal::Ctx;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::{prelude::*, Clamped, JsCast};
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, ImageData, KeyboardEvent, MouseEvent};
 
type FrameCallback = Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>>;
 
pub fn run<G: GameState>(mut ctx: Ctx, mut gs: G) -> super::BError {
    let window = web_sys::window().ok_or("no window")?;
    let document = window.document().ok_or("no document")?;
    let canvas: HtmlCanvasElement = document
        .get_element_by_id("canvas")
        .ok_or("no <canvas id=\"canvas\"> on the page")?
        .dyn_into()
        .map_err(|_| "#canvas is not a canvas")?;
    let (w, h) = (ctx.screen().width, ctx.screen().height);
    canvas.set_width(w as u32);
    canvas.set_height(h as u32);
    let ctx2d: CanvasRenderingContext2d = canvas
        .get_context("2d")
        .map_err(|_| "no 2d context")?
        .ok_or("no 2d context")?
        .dyn_into()
        .map_err(|_| "not a 2d context")?;
 
    let input = Rc::new(RefCell::new(InputQueue::default()));
    let modifiers = Rc::new(RefCell::new((false, false, false)));
    let mouse = Rc::new(RefCell::new((0i32, 0i32)));
 
    {
        let (input, modifiers) = (input.clone(), modifiers.clone());
        let on_key = Closure::<dyn FnMut(KeyboardEvent)>::new(move |e: KeyboardEvent| {
            *modifiers.borrow_mut() = (e.shift_key(), e.ctrl_key(), e.alt_key());
            if let Some(k) = map_code(&e.code()) {
                input.borrow_mut().keys.push_back(k);
            }
        });
        window.add_event_listener_with_callback("keydown", on_key.as_ref().unchecked_ref()).map_err(|_| "keydown")?;
        on_key.forget();
    }
    {
        let modifiers = modifiers.clone();
        let on_key_up = Closure::<dyn FnMut(KeyboardEvent)>::new(move |e: KeyboardEvent| {
            *modifiers.borrow_mut() = (e.shift_key(), e.ctrl_key(), e.alt_key());
        });
        window.add_event_listener_with_callback("keyup", on_key_up.as_ref().unchecked_ref()).map_err(|_| "keyup")?;
        on_key_up.forget();
    }
    
    for (event, down) in [("mousemove", None), ("mousedown", Some(true)), ("mouseup", Some(false))] {
        let (input, mouse) = (input.clone(), mouse.clone());
        let cb = Closure::<dyn FnMut(MouseEvent)>::new(move |e: MouseEvent| {
            *mouse.borrow_mut() = (e.offset_x(), e.offset_y());
            if let Some(d) = down {
                if e.button() == 0 {
                    input.borrow_mut().mouse_down = d;
                }
            }
        });
        canvas.add_event_listener_with_callback(event, cb.as_ref().unchecked_ref()).map_err(|_| "mouse")?;
        cb.forget();
    }
 
    let mut rgba = Vec::with_capacity(w * h * 4);
    let mut last: Option<f64> = None;
    let frame: FrameCallback = Rc::new(RefCell::new(None));
    let next = frame.clone();
    let win = window.clone();
    *frame.borrow_mut() = Some(Closure::new(move |now: f64| {
        ctx.frame_time_ms = last.map_or(16.0, |l| (now - l) as f32);
        last = Some(now);
        (ctx.shift, ctx.control, ctx.alt) = *modifiers.borrow();
        ctx.mouse_px = *mouse.borrow();
        input.borrow_mut().apply(&mut ctx);
 
        gs.tick(&mut ctx);
        ctx.render().write_rgba(&mut rgba);
        match ImageData::new_with_u8_clamped_array_and_sh(Clamped(&rgba), w as u32, h as u32) {
            Ok(img) => {
                let _ = ctx2d.put_image_data(&img, 0.0, 0.0);
            }
            Err(e) => web_sys::console::error_1(&e),
        }
        if let Some(cb) = next.borrow().as_ref() {
            let _ = win.request_animation_frame(cb.as_ref().unchecked_ref());
        }
    }));
    window
        .request_animation_frame(frame.borrow().as_ref().unwrap().as_ref().unchecked_ref())
        .map_err(|_| "requestAnimationFrame")?;
    std::mem::forget(frame);
    Ok(())
}

fn map_code(code: &str) -> Option<Key> {
    if let Some(l) = code.strip_prefix("Key") {
        let c = l.chars().next()?;
        return Key::from_letter(c);
    }
    Some(match code {
        "Digit0" => Key::Key0, "Digit1" => Key::Key1, "Digit2" => Key::Key2, "Digit3" => Key::Key3,
        "Digit4" => Key::Key4, "Digit5" => Key::Key5, "Digit6" => Key::Key6, "Digit7" => Key::Key7,
        "Digit8" => Key::Key8, "Digit9" => Key::Key9,
        "Numpad0" => Key::Numpad0, "Numpad1" => Key::Numpad1, "Numpad2" => Key::Numpad2,
        "Numpad3" => Key::Numpad3, "Numpad4" => Key::Numpad4, "Numpad5" => Key::Numpad5,
        "Numpad6" => Key::Numpad6, "Numpad7" => Key::Numpad7, "Numpad8" => Key::Numpad8,
        "Numpad9" => Key::Numpad9, "NumpadAdd" => Key::NumpadAdd, "NumpadSubtract" => Key::NumpadSubtract,
        "NumpadMultiply" => Key::NumpadMultiply, "NumpadDivide" => Key::NumpadDivide,
        "NumpadDecimal" => Key::NumpadDecimal, "NumpadEnter" => Key::NumpadEnter,
        "F1" => Key::F1, "F2" => Key::F2, "F3" => Key::F3, "F4" => Key::F4, "F5" => Key::F5, "F6" => Key::F6,
        "F7" => Key::F7, "F8" => Key::F8, "F9" => Key::F9, "F10" => Key::F10, "F11" => Key::F11, "F12" => Key::F12,
        "ArrowUp" => Key::Up, "ArrowDown" => Key::Down, "ArrowLeft" => Key::Left, "ArrowRight" => Key::Right,
        "Enter" => Key::Return, "Escape" => Key::Escape, "Space" => Key::Space, "Tab" => Key::Tab,
        "Backspace" => Key::Back, "Insert" => Key::Insert, "Delete" => Key::Delete, "Home" => Key::Home,
        "End" => Key::End, "PageUp" => Key::PageUp, "PageDown" => Key::PageDown,
        "Comma" => Key::Comma, "Period" => Key::Period, "Slash" => Key::Slash, "Backslash" => Key::Backslash,
        "Semicolon" => Key::Semicolon, "Quote" => Key::Apostrophe, "Minus" => Key::Minus, "Equal" => Key::Equals,
        "BracketLeft" => Key::LBracket, "BracketRight" => Key::RBracket, "Backquote" => Key::Grave,
        _ => return None,
    })
}