use std::collections::HashMap;
use rltk::{FontCharType, VirtualKeyCode, RGB};
use specs::prelude::*;
use crate::{Map, ParticleLifetime, Pools, Position, Renderable};

pub const MOVE_SECS: f32 = 0.09;
pub const BUMP_SECS: f32 = 0.14;

pub const CATCHUP_SECS: f32 = 0.15;

pub const BUMP_DISTANCE: f32 = 0.45;
pub const FLASH_SECS: f32 = 0.20;
pub const TEXT_SECS: f32 = 0.9;
pub const GHOST_SECS: f32 = 0.35;

pub const MAX_TWEEN_TILES: i32 = 3;
pub const INPUT_EARLY_SECS: f32 = 0.045;

#[derive(Clone, Copy, Debug)]
pub enum GameEvent {
    Attack {attacker: Entity, target: Entity, damage: Option<i32>}
}

#[derive(Default)]
pub struct Events {
    queue: Vec<GameEvent>
}

impl Events {
    pub fn push(&mut self, event: GameEvent) {
        self.queue.push(event);
    }
}

#[derive(Clone)]
pub enum EffectKind {
    Text {text: String, color: RGB},
    Ghost { glyph: FontCharType, fg: RGB, bg: RGB}
}

#[derive(Clone)]
pub struct Effect {
    pub kind: EffectKind,
    pub x: f32,
    pub y: f32,
    pub age: f32,
    pub life: f32
}

impl Effect {
    pub fn progress(&self) -> f32 {
        (self.age / self.life).clamp(0.0, 1.0)
    }
}

struct Visual {
    pos: (f32, f32),
    to: (i32, i32),
    bump_dir: (f32, f32),
    bump_elapsed: f32,
    flash: f32,
    last_hp: Option<i32>,
    suppress_hp_text: bool,
    seen: u64,
    creature: bool,
    glyph: FontCharType,
    fg: RGB,
    bg: RGB
}

impl Visual {
    fn snapped(x: i32, y: i32) -> Self {
        Visual {
            pos: (x as f32, y as f32), to: (x, y),
            bump_dir: (0.0, 0.0), bump_elapsed: BUMP_SECS, flash: 0.0,
            last_hp: None, suppress_hp_text: false, seen: 0, creature: false,
            glyph: 0, fg: RGB::from_f32(1.0, 1.0, 1.0), bg: RGB::from_f32(0.0, 0.0, 0.0),
        }
    }

    fn lag(&self) -> f32 {
        let (dx, dy) = (self.to.0 as f32 - self.pos.0, self.to.1 as f32 - self.pos.1);
        (dx * dx + dy * dy).sqrt()
    }

    fn step(&mut self, dt: f32) {
        let (dx, dy) = (self.to.0 as f32 - self.pos.0, self.to.1 as f32 - self.pos.1);
        let dist = (dx * dx + dy * dy).sqrt();
        if dist <= 1e-4 {
            self.pos = (self.to.0 as f32, self.to.1 as f32);
            return;
        }
        let speed = (1.0 / MOVE_SECS).max(dist / CATCHUP_SECS);
        let travel = speed * dt;
        if travel >= dist {
            self.pos = (self.to.0 as f32, self.to.1 as f32);
        } else {
            self.pos = (self.pos.0 + dx / dist * travel, self.pos.1 + dy / dist * travel);
        }
    }

    fn base_pos(&self) -> (f32, f32) {
        self.pos
    }

    fn pos(&self) -> (f32, f32) {
        let (x, y) = self.base_pos();
        if self.bump_elapsed < BUMP_SECS {
            let k = (self.bump_elapsed / BUMP_SECS * std::f32::consts::PI).sin() * BUMP_DISTANCE;
            (x + self.bump_dir.0 * k, y + self.bump_dir.1 * k)
        } else {
            (x, y)
        }
    }

    fn blocking_remaining(&self) -> f32 {
        let moving = self.lag() * MOVE_SECS;
        let bumping = (BUMP_SECS - self.bump_elapsed).max(0.0);
        moving.max(bumping)
    }
}

#[derive(Default)]
pub struct Presentation {
    visuals: HashMap<Entity, Visual>,
    effects: Vec<Effect>,
    camera: (f32, f32),
    camera_valid: bool,
    map_signature: (i32, i32, i32),
    stamp: u64,
    player: Option<Entity>,
    buffered_key: Option<VirtualKeyCode>
}

impl Presentation {
    pub fn reset(&mut self) {
        self.visuals.clear();
        self.effects.clear();
        self.camera_valid = false;
        self.buffered_key = None;
    }

    pub fn visual_pos(&self, entity: Entity) -> Option<(f32, f32)> {
        self.visuals.get(&entity).map(Visual::pos)
    }

    pub fn flash(&self, entity: Entity) -> f32 {
        self.visuals.get(&entity).map_or(0.0, |v| (v.flash / FLASH_SECS).clamp(0.0, 1.0))
    }

    pub fn effects(&self) -> &[Effect] {
        &self.effects
    }

    pub fn camera(&self) -> (f32, f32) {
        self.camera
    }

    pub fn camera_valid(&self) -> bool {
        self.camera_valid
    }

    pub fn input_ready(&self) -> bool {
        match self.player.and_then(|p| self.visuals.get(&p)) {
            Some(v) => v.blocking_remaining() <= INPUT_EARLY_SECS,
            None => true
        }
    }

    pub fn filter_key(&mut self, key: Option<VirtualKeyCode>, ready: bool) -> Option<VirtualKeyCode> {
        if !ready {
            if key.is_some() {self.buffered_key = key;}
            return None;
        }
        match key {
            Some(k) => {self.buffered_key = None; Some(k)}
            None => self.buffered_key.take()
        }
    }

    pub fn update(&mut self, dt: f32) {
        let dt = dt.clamp(0.0, 0.1);
        for v in self.visuals.values_mut() {
            v.step(dt);
            v.bump_elapsed = (v.bump_elapsed + dt).min(BUMP_SECS);
            v.flash = (v.flash - dt).max(0.0);
        }
        for e in self.effects.iter_mut() { e.age += dt; }
        self.effects.retain(|e| e.age < e.life);

        if let Some(pos) = self.player.and_then(|p| self.visuals.get(&p)).map(Visual::base_pos) {
            self.camera = pos;
            self.camera_valid = true;
        }
    }

    pub fn observe(&mut self, ecs: &World) {
        let signature = match ecs.try_fetch::<Map>() {
            Some(map) => (map.depth, map.width, map.height),
            None => return,
        };
        if signature != self.map_signature {
            self.map_signature = signature;
            self.reset();
        }
        self.player = ecs.try_fetch::<Entity>().map(|e| *e);

        let events = match ecs.try_fetch_mut::<Events>() {
            Some(mut e) => std::mem::take(&mut e.queue),
            None => Vec::new()
        };

        for event in events {
            match event {
                GameEvent::Attack { attacker, target, damage } => self.apply_attack(attacker, target, damage),
            }
        }

        self.stamp += 1;
        let stamp = self.stamp;
        let entities = ecs.entities();
        let positions = ecs.read_storage::<Position>();
        let renderables = ecs.read_storage::<Renderable>();
        let pools = ecs.read_storage::<Pools>();
        let particles = ecs.read_storage::<ParticleLifetime>();

        for (entity, pos, render, _) in (&entities, & positions, &renderables, !&particles).join() {
            let v = self.visuals.entry(entity).or_insert_with(|| Visual::snapped(pos.x, pos.y));
            v.seen = stamp;
            v.glyph = render.glyph;
            v.fg = render.fg;
            v.bg = render.bg;
            v.creature = pools.get(entity).is_some();

            if v.to != (pos.x, pos.y) {
                let far = (pos.x as f32 - v.pos.0).abs().max((pos.y as f32 - v.pos.1).abs());
                if far > MAX_TWEEN_TILES as f32 {
                    v.pos = (pos.x as f32, pos.y as f32);
                }
                v.to = (pos.x, pos.y);
            }

            if let Some(p) = pools.get(entity) {
                let hp = p.hit_points.current;
                if let Some(last) = v.last_hp {
                    let (x, y) = v.pos();
                    if hp < last {
                        v.flash = FLASH_SECS;
                        if !v.suppress_hp_text {
                            self.effects.push(text_effect(format!("-{}", last-hp), RGB::from_f32(1.0, 0.25, 0.25), x, y));
                        }
                    } else if hp > last {
                        self.effects.push(text_effect(format!("+{}", hp-last), RGB::from_f32(0.3, 1.0, 0.3), x, y));
                    }
                }
                v.last_hp = Some(hp);
            }
            v.suppress_hp_text = false;
        }

        let gone: Vec<Entity> = self.visuals.iter().filter(|(_, v)| v.seen != stamp).map(|(e, _)| *e).collect();
        for entity in gone {
            if let Some(v) = self.visuals.remove(&entity) {
                if v.creature {
                    let (x, y) = v.pos();
                    self.effects.push(Effect { 
                        kind: EffectKind::Ghost { glyph: v.glyph, fg: v.fg, bg: v.bg }, 
                        x, y, age: 0.0, life: GHOST_SECS 
                    });
                }
            }
        }
    }

    fn apply_attack(&mut self, attacker: Entity, target: Entity, damage: Option<i32>) {
        let target_pos = self.visuals.get(&target).map(Visual::pos);
        if let (Some(a), Some(tp)) = (self.visuals.get_mut(&attacker), target_pos) {
            let (ax, ay) = a.base_pos();
            let (dx, dy) = (tp.0 - ax, tp.1 - ay);
            let len = (dx * dx + dy * dy).sqrt().max(1e-3);
            a.bump_dir = (dx / len, dy / len);
            a.bump_elapsed = 0.0;
        }
        if let (Some(t), Some((x, y))) = (self.visuals.get_mut(&target), target_pos) {
            t.suppress_hp_text = true;
            match damage {
                Some(d) => {
                    t.flash = FLASH_SECS;
                    self.effects.push(text_effect(format!("-{d}"), RGB::from_f32(1.0, 0.25, 0.25), x, y));
                }
                None => self.effects.push(text_effect("miss".to_string(), RGB::from_f32(0.7, 0.7, 0.7), x, y)),
            }
        }
    }

}

fn text_effect(text: String, color: RGB, x: f32, y: f32) -> Effect {
    Effect { kind: EffectKind::Text { text, color }, x, y, age: 0.0, life: TEXT_SECS }
}