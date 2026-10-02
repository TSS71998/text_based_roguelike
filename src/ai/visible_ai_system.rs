use specs::prelude::*;
use crate::{Chasing, Faction, Map, MyTurn, Position, Viewshed, WantsToApproach, WantsToFlee, raws::{RawMaster, Reaction, faction_reaction}};

pub struct VisibleAI {}

impl<'a> System<'a> for VisibleAI {
    #[allow(clippy::type_complexity)]
    type SystemData = ( ReadStorage<'a, MyTurn>,
                        ReadStorage<'a, Faction>,
                        ReadStorage<'a, Position>,
                        ReadExpect<'a, Map>,
                        WriteStorage<'a, WantsToApproach>,
                        WriteStorage<'a, WantsToFlee>,
                        Entities<'a>,
                        ReadExpect<'a, Entity>,
                        ReadStorage<'a, Viewshed>,
                        WriteStorage<'a, Chasing>);

    fn run(&mut self, data: Self::SystemData) {
        let (turns, factions, positions, map, mut want_approach, mut want_flee, entities, player, viewsheds, mut chasing) = data;

        let raws = crate::raws::RAWS.lock().unwrap();
        let spatial = crate::spatial::lock();

        let mut reactions: Vec<(usize, Reaction, Entity)> = Vec::new();
        let mut flee: Vec<usize> = Vec::new();
        for (entity, _turn, my_faction, pos, viewshed) in (&entities, &turns, &factions, &positions, &viewsheds).join() {
            if entity != *player {
                reactions.clear();
                flee.clear();
                let my_idx = map.xy_idx(pos.x, pos.y);
                for visible_tile in viewshed.visible_tiles.iter() {
                    let idx = map.xy_idx(visible_tile.x, visible_tile.y);
                    if my_idx != idx {
                        evaluate(idx, &spatial, &factions, &my_faction.name, &raws, &mut reactions);
                    }
                }
                

                let mut done = false;
                for reaction in reactions.iter() {
                    match reaction.1 {
                        Reaction::Attack => {
                            want_approach.insert(entity, WantsToApproach { idx: reaction.0 as i32}).expect("Unable to insert");
                            chasing.insert(entity, Chasing { target: reaction.2 }).expect("Unable to insert");
                            done = true;
                        }
                        Reaction::Flee => {
                            flee.push(reaction.0);
                        }
                        _ => {}
                    }
                }
                if !done && !flee.is_empty() {
                    want_flee.insert(entity, WantsToFlee { indices: std::mem::take(&mut flee) }).expect("Unable to insert");
                }
            }
        }
    }
}

fn evaluate(
    idx: usize, 
    spatial: &crate::spatial::SpatialGuard, 
    factions: &ReadStorage<Faction>,
    my_faction: &str, 
    raws: &RawMaster,
    reactions: &mut Vec<(usize, Reaction, Entity)>
) {
    for other_entity in spatial.content(idx) {
        if let Some(faction) = factions.get(other_entity) {
            reactions.push((
                idx,
                faction_reaction(my_faction, &faction.name, raws),
                other_entity
            ));
        }
    }
}