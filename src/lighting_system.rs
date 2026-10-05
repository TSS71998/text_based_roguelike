use std::collections::HashMap;
use rltk::{Point, RGB};
use specs::prelude::*;
use super::{LightSource, Map, Position, Viewshed};

const AMBIENT: (f32, f32, f32) = (0.04, 0.04, 0.06);
const ANIMATION_HZ: f32 = 12.0;
const FLICKER_GAIN: f32 = 0.20;
const FLICKER_RADIUS: f32 = 0.08;
const SOFT_EDGE: f32 = 1.0;
const DEFAULT_VIEW_RANGE: i32 = 8;

fn ambient_for(_map: &Map) -> RGB {
    RGB::from_f32(AMBIENT.0, AMBIENT.1, AMBIENT.2)
}

struct CachedLight {
    x: i32,
    y: i32,
    range: i32,
    seen: u64,
    tiles: Vec<(u32, f32)>
}

struct ActiveLight {
    entity: Entity,
    x: i32,
    y: i32,
    color: RGB,
    range: i32,
    flicker: f32,
    phase: f32,
}

#[derive(Default)]
pub struct LightingState {
    cache: HashMap<Entity, CachedLight>,
    active: Vec<ActiveLight>,
    env_hash: u64,
    map_signature: (i32, i32, i32),
    stamp: u64,
    clock: f32,
    last_step: u64,
    animated: bool,
    pub masks_computed: u64
}

impl LightingState {
    fn refresh(&mut self, map: &mut Map, lights: Vec<ActiveLight>, viewer: Point, viewer_range: i32) {
        let env = environment_hash(map);
        if env !=self.env_hash {
            self.env_hash = env;
            self.cache.clear();
        }
        self.map_signature = (map.width, map.height, map.depth);

        self.stamp += 1;
        self.active.clear();
        for light in lights {
            let reach = viewer_range + light.range + 2;
            if (light.x - viewer.x).abs() > reach || (light.y - viewer.y).abs() > reach {
                continue;
            }
            let stale = match self.cache.get(&light.entity) {
                Some(c) => c.x != light.x || c.y != light.y || c.range != light.range,
                None => true,
            };

            if stale {
                let tiles = compute_mask(map, light.x, light.y, light.range);
                self.masks_computed += 1;
                self.cache.insert(light.entity, CachedLight { x: light.x, y: light.y, range: light.range, seen: 0, tiles });
            }
            if let Some(c) = self.cache.get_mut(&light.entity) {c.seen = self.stamp;}
            self.active.push(light);
        }
        let stamp = self.stamp;
        self.cache.retain(|_, c| c.seen == stamp);

        self.animated = self.active.iter().any(|l| l.flicker > 0.0);
        self.paint(map);
    }

    fn paint(&self, map: &mut Map) {
        let tile_count = map.tiles.len();
        let ambient = ambient_for(map);
        if map.light.len() != tile_count {
            map.light.resize(tile_count, ambient);
        }
        let Map { light, visible_tiles, .. } = map;
        light.fill(ambient);

        for l in &self.active {
            let Some(cached) = self.cache.get(&l.entity) else {continue};
            let n = if l.flicker > 0.0 {flicker_noise(self.clock, l.phase) } else { 0.0 };
            let radius = (l.range as f32 + SOFT_EDGE) * (1.0 + l.flicker * FLICKER_RADIUS * n);
            let gain = 1.0 + l.flicker * FLICKER_GAIN * n;
            let inv_radius = 1.0 / radius;
            for &(idx, dist) in &cached.tiles {
                let idx = idx as usize;
                if idx >= tile_count || !visible_tiles[idx] {continue;}
                let t = dist * inv_radius;
                if t >= 1.0 { continue;}
                let intensity = (1.0 - t*t) * gain;
                let c = &mut light[idx];
                c.r += l.color.r * intensity;
                c.g += l.color.g * intensity;
                c.b += l.color.b * intensity;
            }
        }
    }
}

pub fn animate(ecs: &World, frame_ms: f32) {
    let mut state = ecs.write_resource::<LightingState>();
    state.clock += frame_ms / 1000.0;
    if !state.animated { return;}
    let step = (state.clock * ANIMATION_HZ) as u64;
    if step == state.last_step { return; }

    let mut map = ecs.write_resource::<Map>();
    if map.outdoors || state.map_signature != (map.width, map.height, map.depth) {return;}

    {
        let entities = ecs.entities();
        let positions = ecs.read_storage::<Position>();
        let valid = state.active.iter().all(|l| {
            entities.is_alive(l.entity) && positions.get(l.entity).map_or(false, |p| p.x == l.x && p.y == l.y)
        });
        if !valid { return; }
    }
    state.last_step = step;
    state.paint(&mut map);   
}

fn flicker_noise(t: f32, phase: f32) -> f32 {
    ((t * 7.3 + phase).sin() + 0.6 * (t * 12.9 + phase * 1.7).sin() + 0.4 * (t * 3.1 + phase * 0.3).sin()) / 2.0
}

fn compute_mask(map: &Map, x:i32, y: i32, range: i32) -> Vec<(u32, f32)> {
    let max_radius = (range as f32 + SOFT_EDGE) * (1.0 + FLICKER_RADIUS);
    let fov_radius = max_radius.ceil() as i32;
    let origin = Point::new(x, y);
    rltk::field_of_view(origin, fov_radius, map)
        .into_iter()
        .filter(|p| p.x > 0 && p.x < map.width - 1 && p.y > 0 && p.y < map.height - 1)
        .filter_map(|p| {
            let (dx, dy) = ((p.x - x) as f32, (p.y - y) as f32);
            let dist = (dx * dx + dy * dy).sqrt();
            if dist < max_radius {Some((map.xy_idx(p.x, p.y) as u32, dist))} else { None }
        })
        .collect()
}

fn environment_hash(map: &Map) -> u64 {
    const PRIME: u64 = 0x100000001b3;
    let mut h: u64 = 0xcbf29ce484222325;
    let mut mix = |v: u64| { h = (h ^ v).wrapping_mul(PRIME); };
    mix(map.width as u64);
    mix(map.height as u64);
    mix(map.depth as u64);
    for t in &map.tiles { mix(*t as u8 as u64); }
    for &b in &map.view_blocked { mix(b as u64); }
    h
}

pub struct LightingSystem {}

impl<'a> System<'a> for LightingSystem {
    #[allow(clippy::type_complexity)]
    type SystemData = ( Entities<'a>,
                        WriteExpect<'a, Map>,
                        WriteExpect<'a, LightingState>,
                        ReadExpect<'a, Entity>,
                        ReadExpect<'a, Point>,
                        ReadStorage<'a, Viewshed>,
                        ReadStorage<'a, Position>,
                        ReadStorage<'a, LightSource>);
    
    fn run(&mut self, data: Self::SystemData) {
        let (entities, mut map, mut state, player, player_pos, viewsheds, positions, lights) = data;

        if map.outdoors {
            state.active.clear();
            state.animated = false;
            return;
        }

        let viewer_range = viewsheds.get(*player).map_or(DEFAULT_VIEW_RANGE, |v| v.range);
        let active: Vec<ActiveLight> = (&entities, &positions, &lights).join()
            .map(|(entity, pos, light)| ActiveLight {
                entity,
                x: pos.x,
                y: pos.y,
                color: light.color,
                range: light.range.max(0),
                flicker: light.flicker.clamp(0.0, 1.0),
                phase: entity.id() as f32 * 1.618,
            })
            .collect();
        
        state.refresh(&mut map, active, *player_pos, viewer_range);
    }
}