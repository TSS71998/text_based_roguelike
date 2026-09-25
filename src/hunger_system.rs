use specs::prelude::*;
use super::{HungerClock, HungerState, RunState, SufferDamage, gamelog::GameLog, MyTurn};

pub struct HungerSystem {}

impl<'a> System<'a> for HungerSystem {
    #[allow(clippy::type_complexity)]
    type SystemData = (
                        Entities<'a>,
                        WriteStorage<'a, HungerClock>,
                        ReadExpect<'a, Entity>,
                        ReadExpect<'a, RunState>,
                        WriteStorage<'a, SufferDamage>,
                        WriteExpect<'a, GameLog>,
                        ReadStorage<'a, MyTurn>
                        );
    
    fn run(&mut self, data: Self::SystemData) {
        let (entities, mut hunger_clock, player_entity, _runstate, mut inflict_damage, mut log, turns) = data;

        for (entity, clock, _myturn) in (&entities, &mut hunger_clock, &turns).join() {
            clock.duration -= 1;
            if clock.duration < 1 {
                match clock.state {
                    HungerState::WellFed => {
                        clock.state = HungerState::Normal;
                        clock.duration = 200;
                        if entity == *player_entity {
                            log.entries.push("You are no longer well fed.".to_string());
                        }
                    }
                    HungerState::Normal => {
                        clock.state = HungerState::Hungry;
                        clock.duration = 200;
                        if entity == *player_entity {
                            log.entries.push("You are hungry".to_string());
                        }
                    }
                    HungerState::Hungry => {
                        clock.state = HungerState::Starving;
                        if entity == *player_entity {
                            log.entries.push("You are starving".to_string());
                        }
                    }
                    HungerState::Starving => {
                        if entity == *player_entity {
                            log.entries.push("Your hunger pains you. You suffer 1 hp damage.".to_string());
                        }
                        SufferDamage::new_damage(&mut inflict_damage, entity, 1, false);
                    }
                }
            }
        }
    }
}