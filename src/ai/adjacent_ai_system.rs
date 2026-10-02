use specs::prelude::*;
use crate::{Faction, Map, MyTurn, Position, WantsToMelee, raws::{RawMaster, Reaction, faction_reaction}};

pub struct AdjacentAI {}

impl<'a> System<'a> for AdjacentAI {
    #[allow(clippy::type_complexity)]
    type SystemData = ( WriteStorage<'a, MyTurn>,
                        ReadStorage<'a, Faction>,
                        ReadStorage<'a, Position>,
                        ReadExpect<'a, Map>,
                        WriteStorage<'a, WantsToMelee>,
                        Entities<'a>,
                        ReadExpect<'a, Entity>);

    fn run(&mut self, data: Self::SystemData) {
        let (mut turns, factions, positions, map, mut want_melee, entities, player) = data;

        let raws = crate::raws::RAWS.lock().unwrap();
        let spatial = crate::spatial::lock();

        let mut turn_done: Vec<Entity> = Vec::new();
        let mut reactions: Vec<(Entity, Reaction)> = Vec::new();
        for (entity, _turn, my_faction, pos) in (&entities, &turns, &factions, &positions).join() {
            if entity != *player {
                let idx = map.xy_idx(pos.x, pos.y);
                let w = map.width;
                let h = map.height;
                let wu = w as usize;
                let name = &my_faction.name;
                if pos.x > 0 {evaluate(idx - 1, &spatial, &factions, name, &raws, &mut reactions);}
                if pos.x < w - 1 {evaluate(idx + 1, &spatial, &factions, name, &raws, &mut reactions);}
                if pos.y > 0 {evaluate(idx - wu, &spatial, &factions, name, &raws, &mut reactions);}
                if pos.y < h - 1 {evaluate(idx + wu, &spatial, &factions, name, &raws, &mut reactions);}
                if pos.y > 0 && pos.x > 0 {evaluate((idx - wu) - 1, &spatial, &factions, name, &raws, &mut reactions);}
                if pos.y > 0 && pos.x < w - 1 {evaluate((idx - wu) + 1, &spatial, &factions, name, &raws, &mut reactions);}
                if pos.y < h - 1 && pos.x > 0 {evaluate((idx + wu) - 1, &spatial, &factions, name, &raws, &mut reactions);}
                if pos.y < h - 1 && pos.x < w - 1 {evaluate((idx + wu) + 1, &spatial, &factions, name, &raws, &mut reactions);}
                let mut done = false;
                for reaction in reactions.iter() {
                    if let Reaction::Attack = reaction.1 {
                        want_melee.insert(entity, WantsToMelee { target: reaction.0 }).expect("Unable to insert melee");
                        done = true;
                    }
                }
                if done {turn_done.push(entity);}
            }
        }

        for done in turn_done.iter() {
            turns.remove(*done);
        }
    }
}

fn evaluate(
    idx: usize, 
    spatial: &crate::spatial::SpatialGuard,
    factions: &ReadStorage<Faction>, 
    my_faction: &str, 
    raws: &RawMaster,
    reactions: &mut Vec<(Entity, Reaction)>
) {
    for other_entity in spatial.content(idx) {
        if let Some(faction) = factions.get(other_entity) {
            reactions.push((
                other_entity,
                faction_reaction(my_faction, &faction.name, raws)
            ));
        }
    };
}