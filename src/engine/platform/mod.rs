use super::terminal::Ctx;

#[cfg(not(target_arch = "wasm32"))]
mod desktop;
#[cfg(target_arch = "wasm32")]
mod web;

pub type BError = Result<(), Box<dyn std::error::Error>>;

pub trait GameState: 'static {
    fn tick(&mut self, ctx: &mut Ctx);
}

pub fn main_loop<G: GameState>(ctx: Ctx, gs: G) -> BError {
    #[cfg(not(target_arch = "wasm32"))]
    return desktop::run(ctx, gs);
    #[cfg(target_arch = "wasm32")]
    return web::run(ctx, gs);
}

pub fn log(msg: impl AsRef<str>) {
    #[cfg(not(target_arch = "wasm32"))]
    eprintln!("{}", msg.as_ref());
    #[cfg(target_arch = "wasm32")]
    web_sys::console::log_1(&msg.as_ref().into());
}

pub fn entropy_seed() -> u64 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        let local = 0u8;
        nanos ^ (&local as *const u8 as u64).rotate_left(32)
    }
    #[cfg(target_arch = "wasm32")]
    {
        let a = (js_sys::Math::random() * 4_294_967_296.0) as u64;
        let b = (js_sys::Math::random() * 4_294_967_296.0) as u64;
        (a << 32) ^ b ^ js_sys::Date::now() as u64
    }
}

#[derive(Default)]
struct InputQueue {
    keys: std::collections::VecDeque<super::input::Key>,
    mouse_down: bool,
    was_down: bool
}

impl InputQueue {
    fn apply(&mut self, ctx: &mut Ctx) {
        ctx.key = self.keys.pop_front();
        ctx.left_click = self.mouse_down && !self.was_down;
        self.was_down = self.mouse_down;
        if self.keys.len() > 8 {
            self.keys.truncate(8);
        }
    }
}