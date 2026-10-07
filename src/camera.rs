use std::cell::Cell;
use specs::prelude::*;
use super::{Map, Position, Renderable, Hidden};
use crate::map::tile_glyph;
use crate::presentation::{EffectKind, Presentation};
use rltk::{Degrees, Point, PointF, Rltk, RGB, RGBA};

const SHOW_BOUNDARIES: bool = true;

pub const VIEW_W: i32 = 48;
pub const VIEW_H: i32 = 44;

pub const MAP_LAYER: usize = 0;
pub const WORLD_LAYER: usize = 1;
pub const UI_LAYER: usize = 2;

const TILE_PX: f32 = 16.0;
const MAP_TILE_SCALE: f32 = 1.0;

const Z_MAP: i32 = 0;
const Z_ACTOR: i32 = 10_000;
const Z_EFFECT: i32 = 20_000;
const Z_HIGHLIGHT: i32 = 30_000;

thread_local! {
    static FRAME_FRAC: Cell<(f32, f32)> = Cell::new((0.0, 0.0));
}

fn snap(v: f32) -> f32 {
    (v * TILE_PX).round() / TILE_PX
}

pub fn clear_layers(ctx: &mut Rltk) {
    for layer in [MAP_LAYER, WORLD_LAYER, UI_LAYER] {
        ctx.set_active_console(layer);
        ctx.cls();
    }
    ctx.set_active_console(UI_LAYER);
}

pub fn map_set_bg(ctx: &mut Rltk, x: i32, y: i32, color: RGB) {
    let (fx, fy) = FRAME_FRAC.with(|f| f.get());
    ctx.set_active_console(WORLD_LAYER);
    ctx.set_fancy(PointF::new(x as f32 - fx, y as f32 - fy + 1.0), Z_HIGHLIGHT, Degrees::new(0.0),
                  PointF::new(1.0, 1.0), RGBA::from_f32(0.0, 0.0, 0.0, 0.0),
                  RGBA::from_f32(color.r, color.g, color.b, 0.55), rltk::to_cp437(' '));
    ctx.set_active_console(UI_LAYER);
}

fn view_origin(ecs: &World) -> (i32, i32, f32, f32) {
    let center = match ecs.try_fetch::<Presentation>() {
        Some(p) if p.camera_valid() => p.camera(),
        _ => {
            let player = ecs.fetch::<Point>();
            (player.x as f32, player.y as f32)
        }
    };
    let min_x = center.0 - (VIEW_W / 2) as f32;
    let min_y = center.1 - (VIEW_H / 2) as f32;
    let (bx, by) = (min_x.floor(), min_y.floor());
    (bx as i32, by as i32, min_x - bx, min_y - by)
}

fn with_alpha(c: RGB, a: f32) -> RGBA {
    RGBA::from_f32(c.r, c.g, c.b, a)
}

pub fn render_camera(ecs: &World, ctx: &mut Rltk) {
    let map = ecs.fetch::<Map>();
    let (base_x, base_y, frac_x, frac_y) = view_origin(ecs);
    let (fx, fy) = (snap(frac_x), snap(frac_y));
    FRAME_FRAC.with(|f| f.set((fx, fy)));

    let origin_x = base_x as f32 + fx;
    let origin_y = base_y as f32 + fy;

    ctx.set_active_console(WORLD_LAYER);

    let map_width = map.width - 1;
    let map_height = map.height - 1;
    let tile_scale = PointF::new(MAP_TILE_SCALE, MAP_TILE_SCALE);
    for sy in 0..=VIEW_H {
        for sx in 0..=VIEW_W {
            let (tx, ty) = (base_x + sx, base_y + sy);
            let drawn = if tx > 0 && tx < map_width && ty > 0 && ty < map_height {
                let idx = map.xy_idx(tx, ty);
                if map.revealed_tiles[idx] { Some(tile_glyph(idx, &*map)) } else { None }
            } else if SHOW_BOUNDARIES {
                Some((rltk::to_cp437('.'), RGB::named(rltk::GRAY), RGB::named(rltk::BLACK)))
            } else {
                None
            };
            if let Some((glyph, fg, bg)) = drawn {
                ctx.set_fancy(PointF::new(sx as f32 - fx, sy as f32 - fy + 1.0), Z_MAP, Degrees::new(0.0),
                              tile_scale, with_alpha(fg, 1.0), with_alpha(bg, 1.0), glyph);
            }
        }
    }

    let presentation = ecs.try_fetch::<Presentation>();
    let entities = ecs.entities();
    let positions = ecs.read_storage::<Position>();
    let renderables = ecs.read_storage::<Renderable>();
    let hidden = ecs.read_storage::<Hidden>();

    let mut actors = (&entities, &positions, &renderables, !&hidden)
        .join()
        .filter(|(_, pos, _, _)| map.visible_tiles[map.xy_idx(pos.x, pos.y)])
        .collect::<Vec<_>>();
    actors.sort_by(|a, b| b.2.render_order.cmp(&a.2.render_order)); // lowest order last = on top

    let mut z = Z_ACTOR;
    for (entity, pos, render, _) in actors {
        let (vx, vy) = presentation.as_ref()
            .and_then(|p| p.visual_pos(entity))
            .unwrap_or((pos.x as f32, pos.y as f32));
        let (sx, sy) = (snap(vx - origin_x), snap(vy - origin_y));
        if sx < -1.0 || sx > (VIEW_W + 1) as f32 || sy < -1.0 || sy > (VIEW_H + 1) as f32 { continue; }

        let flash = presentation.as_ref().map_or(0.0, |p| p.flash(entity));
        let fg = render.fg.lerp(RGB::from_f32(1.0, 1.0, 1.0), flash);
        ctx.set_fancy(PointF::new(sx, sy + 1.0), z, Degrees::new(0.0), PointF::new(1.0, 1.0),
                      with_alpha(fg, 1.0), with_alpha(render.bg, 1.0), render.glyph);
        z += 1;
    }

    if let Some(p) = presentation.as_ref() {
        let mut z = Z_EFFECT;
        for effect in p.effects() {
            let (ex, ey) = (effect.x.round() as i32, effect.y.round() as i32);
            if ex < 0 || ey < 0 || ex >= map.width || ey >= map.height { continue; }
            if !map.visible_tiles[map.xy_idx(ex, ey)] { continue; }
            let t = effect.progress();
            let (sx, sy) = (snap(effect.x - origin_x), snap(effect.y - origin_y));
            match &effect.kind {
                EffectKind::Ghost { glyph, fg, bg: _ } => {
                    ctx.set_fancy(PointF::new(sx, sy + 1.0), z, Degrees::new(0.0), PointF::new(1.0, 1.0),
                                  with_alpha(*fg, 1.0 - t), RGBA::from_f32(0.0, 0.0, 0.0, 0.0), *glyph);
                }
                EffectKind::Text { text, color } => {
                    let rise = t * 0.9 + 0.2;
                    let alpha = 1.0 - t * t;
                    for (i, ch) in text.chars().enumerate() {
                        ctx.set_fancy(PointF::new(sx + i as f32 * 0.5, sy + 1.0 - rise), z,
                                      Degrees::new(0.0), PointF::new(1.0, 1.2),
                                      with_alpha(*color, alpha), RGBA::from_f32(0.0, 0.0, 0.0, 0.0), rltk::to_cp437(ch));
                    }
                }
            }
            z += 1;
        }
    }

    ctx.set_active_console(UI_LAYER);
}


pub fn get_screen_bounds(ecs: &World, _ctx: &mut Rltk) -> (i32, i32, i32, i32) {
    let (min_x, min_y, _, _) = view_origin(ecs);
    (min_x, min_x + VIEW_W, min_y, min_y + VIEW_H)
}

pub fn render_debug_map(map: &Map, ctx: &mut Rltk) {
    ctx.set_active_console(MAP_LAYER);
    ctx.set_offset(0.0, 0.0);
    let player_pos = Point::new(map.width / 2, map.height / 2);
    let (x_chars, y_chars) = ctx.get_char_size();

    let center_x = (x_chars / 2) as i32;
    let center_y = (y_chars / 2) as i32;

    let min_x = player_pos.x - center_x;
    let min_y = player_pos.y - center_y;
    let max_x = min_x + x_chars as i32;
    let max_y = min_y + y_chars as i32;

    let map_width = map.width - 1;
    let map_height = map.height - 1;

    let mut y = 0;
    for ty in min_y .. max_y {
        let mut x = 0;
        for tx in min_x .. max_x {
            if tx > 0 && tx < map_width && ty > 0 && ty < map_height {
                let idx = map.xy_idx(tx, ty);
                if map.revealed_tiles[idx] {
                    let (glyph, fg, bg) = tile_glyph(idx, &*map);
                    ctx.set(x, y, fg, bg, glyph);
                }
            } else if SHOW_BOUNDARIES {
                ctx.set(x, y, RGB::named(rltk::GRAY), RGB::named(rltk::BLACK), rltk::to_cp437('.'));
            }
            x += 1;
        }
        y += 1;
    }
    ctx.set_active_console(UI_LAYER);
}