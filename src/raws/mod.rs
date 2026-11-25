use serde::{Deserialize};
use std::sync::{LazyLock, Mutex};
mod mob_structs;
use mob_structs::*;
mod item_structs;
use item_structs::*;
mod prop_structs;
use prop_structs::*;
mod spawn_table_structs;
use spawn_table_structs::*;
mod loot_structs;
use loot_structs::*;
mod faction_structs;
pub use faction_structs::*;
mod rawmaster;
pub use rawmaster::*;

const RAW_FILE: &str = include_str!("../../raws/spawns.json");

pub static RAWS: LazyLock<Mutex<RawMaster>> = LazyLock::new(|| Mutex::new(RawMaster::empty()));

#[derive(Deserialize, Debug)]
pub struct Raws {
    pub items: Vec<Item>,
    pub mobs: Vec<Mob>,
    pub props: Vec<Prop>,
    pub spawn_table: Vec<SpawnTableEntry>,
    pub loot_tables: Vec<LootTable>,
    pub faction_table: Vec<FactionInfo>
}

pub fn load_raws() {
    let decoder: Raws = serde_json::from_str(RAW_FILE).expect("Unable to parse JSON");
    
    RAWS.lock().unwrap().load(decoder);
}