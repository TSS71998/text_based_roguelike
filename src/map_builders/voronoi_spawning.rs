use super::{MetaMapBuilder, BuilderMap, TileType, spawner};
use std::collections::HashMap;

pub struct VoronoiSpawning{}

impl MetaMapBuilder for VoronoiSpawning{
    fn build_map(&mut self, rng: &mut crate::engine::RandomNumberGenerator, build_data: &mut BuilderMap) {
        self.build(rng, build_data);
    }
}

impl VoronoiSpawning {
     #[allow(dead_code)]
    pub fn new() -> Box<VoronoiSpawning> {
        Box::new(VoronoiSpawning{})
    }

    #[allow(clippy::map_entry)]
    fn build(&mut self, rng: &mut crate::engine::RandomNumberGenerator, build_data: &mut BuilderMap) {
        const CELL: i32 = 12;
        let (w, h) = (build_data.map.width, build_data.map.height);
        let (gw, gh) = (w / CELL + 2, h / CELL + 2);
        let seeds: Vec<(i32, i32)> = (0..gw * gh)
            .map(|i| ((i % gw) * CELL + rng.range(0, CELL), (i / gw) * CELL + rng.range(0, CELL)))
            .collect();
        let region_of = |x: i32, y: i32| -> i32 {
            let (cx, cy) = (x / CELL, y / CELL);
            let mut best = (i32::MAX, 0);
            for ny in (cy - 1).max(0)..=(cy + 1).min(gh - 1) {
                for nx in (cx - 1).max(0)..=(cx + 1).min(gw - 1) {
                    let id = ny * gw + nx;
                    let (sx, sy) = seeds[id as usize];
                    let d = (sx - x).abs() + (sy - y).abs();
                    if d < best.0 { best = (d, id); }
                }
            }
            best.1
        };

        let mut noise_areas: HashMap<i32, Vec<usize>> = HashMap::new();

        for y in 1 .. h - 1 {
            for x in 1 .. w - 1 {
                let idx = build_data.map.xy_idx(x, y);
                if build_data.map.tiles[idx] == TileType::Floor {
                    noise_areas.entry(region_of(x, y)).or_default().push(idx);
                }
            }
        }
        
        for area in noise_areas.iter() {
            spawner::spawn_region(&build_data.map, rng, area.1, build_data.map.depth, &mut build_data.spawn_list);
        }
    }
}