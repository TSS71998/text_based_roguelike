use crate::engine::{Algorithm2D, BaseMap, Point};
use serde::{Serialize, Deserialize};

mod tiletype;
pub use tiletype::*;
mod themes;
pub use themes::*;
pub mod dungeon;
pub use dungeon::*;

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct Map {
    pub tiles: Vec<TileType>,
    pub width: i32,
    pub height: i32,
    pub revealed_tiles: Vec<bool>,
    pub visible_tiles: Vec<bool>,
    pub depth: i32,
    pub bloodstains: Vec<bool>,
    pub view_blocked: Vec<bool>,
    pub name: String,
    pub outdoors: bool,
    pub light: Vec<crate::engine::RGB>
}

impl Map{
    pub fn xy_idx(&self,x: i32, y: i32) -> usize {
        (y as usize * self.width as usize) + x as usize
    }

    pub fn populate_blocked(&mut self) {
        crate::spatial::populate_blocked_from_map(self);
    }

    pub fn clear_content_index(&mut self) {
        crate::spatial::clear();
    }

    pub fn new<S: ToString>(new_depth: i32, width: i32, height: i32, name: S) -> Map {
        let map_tile_count = (width*height) as usize;
        crate::spatial::set_size(map_tile_count);
        Map {
            tiles: vec![TileType::Wall; map_tile_count],
            width,
            height,
            revealed_tiles: vec![false; map_tile_count],
            visible_tiles: vec![false; map_tile_count],
            depth: new_depth,
            bloodstains: vec![false; map_tile_count],
            view_blocked: vec![false; map_tile_count],
            name: name.to_string(),
            outdoors: true, 
            light: vec![crate::engine::RGB::from_f32(0.0, 0.0, 0.0); map_tile_count]
        }
    }
}

impl Algorithm2D for Map {
    fn dimensions(&self) -> crate::engine::Point {
        Point::new(self.width, self.height)
    }
}

impl BaseMap for Map {
    fn is_opaque(&self, idx: usize) -> bool {
        if idx > 0 && idx < self.tiles.len() {
            tile_opaque(self.tiles[idx]) || self.view_blocked.get(idx).copied().unwrap_or(false)
        } else {
            true
        }
    }

    fn get_pathing_distance(&self, idx1: usize, idx2: usize) -> f32 {
        let w = self.width as usize;
        let p1 = Point::new((idx1 % w) as i32, (idx1 / w) as i32);
        let p2 = Point::new((idx2 % w) as i32, (idx2 / w) as i32);
        crate::engine::DistanceAlg::Manhattan.distance2d(p1, p2)
    }

    fn get_available_exits(&self, idx:usize) -> crate::engine::Exits {
        const DIAGONAL_COST : f32 = 1.5;
        let mut exits = crate::engine::Exits::new();
        let x = idx as i32 % self.width;
        let y = idx as i32 / self.width;
        let tt = self.tiles[idx as usize];
        let w = self.width  as usize;

        let spatial = crate::spatial::lock();
        let valid = |nx: i32, ny: i32| -> bool {
            if nx < 1 || nx > self.width - 1 || ny < 1 || ny > self.height - 1 {return false;}
            !spatial.is_blocked(self.xy_idx(nx, ny))
        };

        // Cardinal directions
        if valid(x-1, y) { exits.push((idx-1, tile_cost(tt))) };
        if valid(x+1, y) { exits.push((idx+1, tile_cost(tt))) };
        if valid(x, y-1) { exits.push((idx-w, tile_cost(tt))) };
        if valid(x, y+1) { exits.push((idx+w, tile_cost(tt))) };

        // Diagonals
        if valid(x-1, y-1) { exits.push(((idx-w)-1, tile_cost(tt) * DIAGONAL_COST)); }
        if valid(x+1, y-1) { exits.push(((idx-w)+1, tile_cost(tt) * DIAGONAL_COST)); }
        if valid(x-1, y+1) { exits.push(((idx+w)-1, tile_cost(tt) * DIAGONAL_COST)); }
        if valid(x+1, y+1) { exits.push(((idx+w)+1, tile_cost(tt) * DIAGONAL_COST)); }

        exits
    }
}