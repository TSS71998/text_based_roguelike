use super::{UI_LAYER, VIEW3D_LAYER, VIEWPORT};
use crate::engine::{Ctx, Direction, Point, RGB, framebuffer::pack};
use crate::{Facing, Map, TileShape, tile_def};
use specs::prelude::*;

const LOOK_AHEAD: i32 = 12;

pub fn render(ecs: &World, ctx: &mut Ctx) {
    let player = *ecs.fetch::<Entity>();
    let pos = *ecs.fetch::<Point>();
    let facing = ecs
        .read_storage::<Facing>()
        .get(player)
        .map_or(Direction::North, |f| f.dir);
    let map = ecs.fetch::<Map>();

    if let Some(fb) = ctx.pixels_mut(VIEW3D_LAYER) {
        let horizon = fb.height / 2;
        for y in 0..fb.height {
            let t = (y as f32 - horizon as f32).abs() / horizon as f32;
            let color = if y < horizon {
                RGB::from_f32(0.18, 0.18, 0.22) * t
            } else {
                RGB::from_f32(0.30, 0.22, 0.14) * t
            };
            fb.pixels[y * fb.width..(y + 1) * fb.width].fill(pack(color));
        }
    }
    ctx.set_layer_visible(VIEW3D_LAYER, true);

    ctx.set_active_console(UI_LAYER);
    let (vx, vy, vw, _) = VIEWPORT;
    let center = |text: &str| vx + (vw - text.len() as i32) / 2;
    let white = RGB::from_f32(1.0, 1.0, 1.0);
    let clear = crate::engine::RGBA::from_f32(0.0, 0.0, 0.0, 0.0);
    let title = "First-person view (placeholder)";
    ctx.print_color(center(title), vy + 2, white, clear, title);
    let heading = format!("Facing {}", facing.name());
    ctx.print_color(
        center(&heading),
        vy + 4,
        RGB::from_f32(1.0, 1.0, 0.0),
        clear,
        &heading,
    );
    let ahead = describe_ahead(&map, pos, facing);
    ctx.print_color(
        center(&ahead),
        vy + 5,
        RGB::from_f32(0.7, 0.7, 0.7),
        clear,
        &ahead,
    );
    let help = "Tab map  Up/Dn step  Lt/Rt turn  Q/E strafe";
    ctx.print_color(
        center(help),
        vy + VIEWPORT.3 - 2,
        RGB::from_f32(0.6, 0.6, 0.6),
        clear,
        help,
    );
}

fn describe_ahead(map: &Map, from: Point, facing: Direction) -> String {
    let (dx, dy) = facing.delta();
    for step in 1..=LOOK_AHEAD {
        let (x, y) = (from.x + dx * step, from.y + dy * step);
        if x < 0 || y < 0 || x >= map.width || y >= map.height {
            return "The edge of the map".to_string();
        }
        let idx = map.xy_idx(x, y);
        if !map.revealed_tiles[idx] {
            return format!("Unexplored, {step} ahead");
        }
        let what = match tile_def(map.tiles[idx]).shape {
            TileShape::Solid => "A wall",
            TileShape::Column => "A rock column",
            TileShape::Liquid => "Water",
            TileShape::StairsDown => "Stairs down",
            TileShape::StairsUp => "Stairs up",
            TileShape::Floor => continue,
        };
        return format!("{what}, {step} ahead");
    }
    "Open ground ahead".to_string()
}
