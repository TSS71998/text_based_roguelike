use super::{Map, TileType};
use crate::engine::RGB;

pub fn tile_glyph(idx: usize, map: &Map) -> (crate::engine::FontCharType, RGB, RGB) {
    let (glyph, mut fg, mut bg) = match map.depth {
        3 => get_limestone_cavern_glyph(idx, map),
        2 => get_forest_glyph(idx, map),
        _ => get_tile_glyph_default(idx, map)
    };
    
    if map.bloodstains.get(idx).copied().unwrap_or(false) {bg = RGB::from_f32(0.75, 0.0, 0.0);}
    if !map.visible_tiles[idx] {
        fg = fg.to_greyscale();
        bg = RGB::from_f32(0., 0., 0.);
    } else if !map.outdoors {
        let l = map.light[idx];
        let light = RGB::from_f32(l.r, l.g, l.b);
        fg = fg * light;
        bg = bg * light;
    }

    (glyph, fg, bg)
}

fn get_limestone_cavern_glyph(idx:usize, map: &Map) -> (crate::engine::FontCharType, RGB, RGB) {
    let glyph;
    let fg;
    let bg = RGB::from_f32(0., 0., 0.);

    match map.tiles[idx] {
        TileType::Wall => { glyph = crate::engine::to_cp437('▒'); fg = RGB::from_f32(0.7, 0.7, 0.7); }
        TileType::Bridge => { glyph = crate::engine::to_cp437('.'); fg = RGB::named(crate::engine::CHOCOLATE); }
        TileType::Road => { glyph = crate::engine::to_cp437('≡'); fg = RGB::named(crate::engine::YELLOW); }
        TileType::Grass => { glyph = crate::engine::to_cp437('"'); fg = RGB::named(crate::engine::GREEN); }
        TileType::ShallowWater => { glyph = crate::engine::to_cp437('░'); fg = RGB::named(crate::engine::CYAN); }
        TileType::DeepWater => { glyph = crate::engine::to_cp437('▓'); fg = RGB::named(crate::engine::BLUE); }
        TileType::Gravel => { glyph = crate::engine::to_cp437(';'); fg = RGB::from_f32(0.5, 0.5, 0.5); }
        TileType::DownStairs => { glyph = crate::engine::to_cp437('>'); fg = RGB::from_f32(0., 1.0, 1.0); }
        TileType::UpStairs => { glyph = crate::engine::to_cp437('<'); fg = RGB::from_f32(0., 1.0, 1.0); }
        TileType::Stalactite => {glyph = crate::engine::to_cp437('╨'); fg = RGB::from_f32(0.5, 0.5, 0.5);}
        TileType::Stalagmite => {glyph = crate::engine::to_cp437('╥'); fg = RGB::from_f32(0.5, 0.5, 0.5);}
        _ => { glyph = crate::engine::to_cp437('░'); fg = RGB::from_f32(0.4, 0.4, 0.4); }
    }

    (glyph, fg, bg)
}

fn get_forest_glyph(idx: usize, map: &Map) -> (crate::engine::FontCharType, RGB, RGB) {
    let glyph;
    let fg;
    let bg = RGB::from_f32(0.0, 0.0, 0.0);

    match map.tiles[idx] {
        TileType::Wall => {glyph = crate::engine::to_cp437('♣'); fg = RGB::from_f32(0., 0.6, 0.);}
        TileType::DownStairs => {glyph = crate::engine::to_cp437('>'); fg = RGB::from_f32(0.0, 1.0, 1.0);}
        TileType::Bridge => {glyph = crate::engine::to_cp437('.'); fg = RGB::named(crate::engine::CHOCOLATE);}
        TileType::Road => {glyph = crate::engine::to_cp437('≡'); fg = RGB::named(crate::engine::YELLOW);}
        TileType::Grass => {glyph = crate::engine::to_cp437('"'); fg = RGB::named(crate::engine::GREEN);}
        TileType::ShallowWater => {glyph = crate::engine::to_cp437('~'); fg = RGB::named(crate::engine::CYAN);}
        TileType::DeepWater => {glyph = crate::engine::to_cp437('≈'); fg = RGB::named(crate::engine::NAVY_BLUE);}
        TileType::Gravel=> {glyph = crate::engine::to_cp437(';'); fg = RGB::from_f32(0.5, 0.5, 0.5);}
        TileType::UpStairs => {glyph = crate::engine::to_cp437('<'); fg = RGB::from_f32(0.0, 1.0, 1.0);}
        _ => {glyph = crate::engine::to_cp437('"'); fg = RGB::from_f32(0.0, 0.6, 0.0);}
    }

    (glyph, fg, bg)
}

fn get_tile_glyph_default(idx: usize, map: &Map) -> (crate::engine::FontCharType, RGB, RGB) {
    let glyph;
    let fg;
    let bg = RGB::from_f32(0.0, 0.0, 0.0);

    match map.tiles[idx] {
        TileType::Floor => {glyph = crate::engine::to_cp437('.'); fg = RGB::from_f32(0.0, 0.5, 0.5);}
        TileType::WoodFloor => {glyph = crate::engine::to_cp437('▒'); fg = RGB::named(crate::engine::CHOCOLATE);}
        TileType::Wall => {
            let x = idx as i32 % map.width;
            let y= idx as i32 / map.width;
            glyph = wall_glyph(&*map, x, y);
            fg = RGB::from_f32(0., 1.0, 0.);
        }
        TileType::DownStairs => {glyph = crate::engine::to_cp437('>'); fg = RGB::from_f32(0.0, 1.0, 1.0);}
        TileType::Bridge => {glyph = crate::engine::to_cp437('.'); fg = RGB::named(crate::engine::CHOCOLATE);}
        TileType::Road => {glyph = crate::engine::to_cp437('~'); fg = RGB::named(crate::engine::GRAY);}
        TileType::Grass => {glyph = crate::engine::to_cp437('"'); fg = RGB::named(crate::engine::GREEN);}
        TileType::ShallowWater => {glyph = crate::engine::to_cp437('≈'); fg = RGB::named(crate::engine::CYAN);}
        TileType::DeepWater => {glyph = crate::engine::to_cp437('≈'); fg = RGB::named(crate::engine::NAVY_BLUE);}
        TileType::Gravel=> {glyph = crate::engine::to_cp437(';'); fg = RGB::named(crate::engine::GRAY);}
        TileType::UpStairs => {glyph = crate::engine::to_cp437('<'); fg = RGB::from_f32(0.0, 1.0, 1.0);}
        TileType::Stalactite => {glyph = crate::engine::to_cp437('╨'); fg = RGB::from_f32(0.5, 0.5, 0.5);}
        TileType::Stalagmite => {glyph = crate::engine::to_cp437('╥'); fg = RGB::from_f32(0.5, 0.5, 0.5);}
    }
    (glyph, fg, bg)
}

fn wall_glyph(map: &Map, x: i32, y: i32) -> crate::engine::FontCharType {
    if x < 1 || x > map.width - 2 || y < 1 || y > map.height - 2 as i32 {return 35;}
    let mut mask: u8 = 0;

    if is_revealed_and_wall(map, x, y - 1) {mask += 1;}
    if is_revealed_and_wall(map, x, y + 1) {mask += 2;}
    if is_revealed_and_wall(map, x - 1, y) {mask += 4;}
    if is_revealed_and_wall(map, x + 1, y) {mask += 8;}

    match mask {
        0 => {9}
        1 => {186}
        2 => {186}
        3 => {186}
        4 => {205}
        5 => {188}
        6 => {187}
        7 => {185}
        8 => {205}
        9 => {200}
        10 => {201}
        11 => {204}
        12 => {205}
        13 => {202}
        14 => {203}
        15 => {206}
        _ => {35}
    }
}

fn is_revealed_and_wall(map: &Map, x: i32, y: i32) -> bool {
    let idx = map.xy_idx(x, y);
    map.tiles[idx] == TileType::Wall && map.revealed_tiles[idx]
}