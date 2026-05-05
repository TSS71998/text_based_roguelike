use rltk::{Rltk, GameState, RGB, VirtualKeyCode};
use specs::prelude::*;
use std::cmp::{max,min};
use specs_derive::Component;

struct State{}
impl GameState for State {
    fn tick(&mut self, ctx: &mut Rltk) {
        ctx.cls();
        ctx.print(1,1,"Hello World")
    }
}

#[derive(Component)]
struct Position{
    x: i32,
    y: i32,
}

fn main() -> rltk::BError {
    use rltk::RltkBuilder;
    let context = RltkBuilder::simple80x50()
        .with_title("Text-Based Roguelike").build()?;
    let gs = State{ };
    rltk::main_loop(context,gs)
}
