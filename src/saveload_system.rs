use std::collections::BTreeMap;
use std::convert::Infallible;
use std::error::Error;
use std::fmt;
use std::fs;
use std::io::{self, BufWriter, Write};
use std::path::Path;

use rltk::Point;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use specs::prelude::*;
use specs::saveload::{DeserializeComponents, SerializeComponents, SimpleMarker, SimpleMarkerAllocator};

use super::components::*;
use super::map::{Map, MasterDungeonMap};

const SAVE_PATH: &str = "./savegame.json";
const SAVE_VERSION: u32 = 1;

#[derive(Debug)]
pub enum SaveError{
    Io(io::Error),
    Format(serde_json::Error),
    UnsupportedVersion {found: u32, expected: u32},
    MissingPlayer,
}

impl fmt::Display for SaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SaveError::Io(e) => write!(f, "file error: {e}"),
            SaveError::Format(e) => write!(f, "corrupt or incompatible save: {e}"),
            SaveError::UnsupportedVersion { found, expected } => {
                write!(f, "save is version {found}, this build read version {expected}")
            }
            SaveError::MissingPlayer => write!(f, "save contains no player"),
        }
    }
}

impl Error for SaveError {}

impl From<io::Error> for SaveError {
    fn from(e: io::Error) -> Self { SaveError::Io(e) }
}

impl From<serde_json::Error> for SaveError {
    fn from(e: serde_json::Error) -> Self { SaveError::Format(e) }
}

#[derive(Serialize)]
struct SaveFileRef<'a> {
    version: u32,
    map: &'a Map,
    dungeon: &'a MasterDungeonMap,
    components: BTreeMap<String, Value>
}

#[derive(Deserialize)]
struct SaveFile {
    map: Map,
    dungeon: MasterDungeonMap,
    components: BTreeMap<String, Value>
}

#[derive(Deserialize)]
struct Header {
    version: u32
}

pub fn does_save_exist() -> bool {
    Path::new(SAVE_PATH).exists()
}

pub fn delete_save() {
    let _ = fs::remove_file(SAVE_PATH);
}

#[cfg(target_arch = "wasm32")]
pub fn save_game(_ecs: &World) -> Result<(), SaveError> { Ok(()) }

#[cfg(not(target_arch = "wasm32"))]
pub fn save_game(ecs: &World) -> Result<(), SaveError> { 
    save_game_to(ecs, SAVE_PATH) 
}

pub fn load_game(ecs: &mut World) -> Result<(), SaveError> {
    load_game_from(ecs, SAVE_PATH)
}

pub fn register_components(ecs: &mut World) {
    register_savable_components(ecs);
}

pub fn save_game_to(ecs: &World, path: impl AsRef<Path>) -> Result<(), SaveError> {
    let path = path.as_ref();
    let map = ecs.fetch::<Map>();
    let dungeon = ecs.fetch::<MasterDungeonMap>();
    let file = SaveFileRef {
        version: SAVE_VERSION,
        map: &map,
        dungeon: &dungeon,
        components: serialize_components(ecs)?
    };

    let tmp = path.with_extension("json.tmp");
    let mut writer = BufWriter::new(fs::File::create(&tmp)?);
    if cfg!(debug_assertions) {
        serde_json::to_writer_pretty(&mut writer, &file)?;
    } else {
        serde_json::to_writer(&mut writer, &file)?;
    }

    writer.flush()?;
    drop(writer);
    fs::rename(&tmp, path)?;
    Ok(())
}

pub fn load_game_from(ecs: &mut World, path: impl AsRef<Path>) -> Result<(), SaveError> {
    let text = fs::read_to_string(path)?;
    let header: Header = serde_json::from_str(&text)?;
    if header.version != SAVE_VERSION {
        return Err(SaveError::UnsupportedVersion { found: header.version, expected: SAVE_VERSION });
    }
    let save: SaveFile = serde_json::from_str(&text)?;
    drop(text);

    let has_player = save.components.get("Player")
        .and_then(Value::as_array)
        .map_or(false, |entries| !entries.is_empty());
    if !has_player {
        return Err(SaveError::MissingPlayer);
    }

    ecs.delete_all();
    ecs.maintain();
    ecs.insert(SimpleMarkerAllocator::<SerializeMe>::new());
    deserialize_components(ecs, save.components)?;

    crate::spatial::set_size(save.map.tiles.len());
    *ecs.write_resource::<Map>() = save.map;
    *ecs.write_resource::<MasterDungeonMap>() = save.dungeon;

    let player = {
        let entries = ecs.entities();
        let players = ecs.read_storage::<Player>();
        let positions = ecs.read_storage::<Position>();
        (&entries, &players, &positions).join().next().map(|(e, _, pos)| (e, Point::new(pos.x, pos.y)))
    };

    let (entity, point) = player.ok_or(SaveError::MissingPlayer)?;
    *ecs.write_resource::<Entity>() = entity;
    *ecs.write_resource::<Point>() = point;

    Ok(())
}

macro_rules! savable_components {
    ($($ty:ident),+ $(,)?) => {
        fn register_savable_components(ecs: &mut World) {
            ecs.register::<SimpleMarker<SerializeMe>>();
            $(ecs.register::<$ty>();)+
        }

        fn serialize_components(ecs: &World) -> Result<BTreeMap<String, Value>, SaveError> {
            let entities = ecs.entities();
            let markers = ecs.read_storage::<SimpleMarker<SerializeMe>>();
            let mut out = BTreeMap::new();
            $(
                let value = SerializeComponents::<Infallible, SimpleMarker<SerializeMe>>::serialize(
                    &(ecs.read_storage::<$ty>(),),
                    &entities,
                    &markers,
                    serde_json::value::Serializer,
                )?;
                out.insert(stringify!($ty).to_string(), value);
            )+
            Ok(out)
        }

        fn deserialize_components(ecs: &World, mut saved: BTreeMap<String, Value>) -> Result<(), SaveError> {
            let entities = ecs.entities();
            let mut markers = ecs.write_storage::<SimpleMarker<SerializeMe>>();
            let mut allocator = ecs.write_resource::<SimpleMarkerAllocator<SerializeMe>>();
            $(
                // A component absent from the file (e.g. added after the save was made) is skipped.
                if let Some(value) = saved.remove(stringify!($ty)) {
                    DeserializeComponents::<Infallible, _>::deserialize(
                        &mut (&mut ecs.write_storage::<$ty>(),),
                        &entities,
                        &mut markers,
                        &mut allocator,
                        value,
                    )?;
                }
            )+
            Ok(())
        }
    };
}

savable_components!(
    Position, Renderable, Player, Viewshed, Monster, Name, BlocksTile, SufferDamage, WantsToMelee,
    Item, Consumable, Ranged, InflictsDamage, AreaEffect, Confusion, ProvidesHealing, InBackpack,
    WantsToPickupItem, WantsToUseItem, WantsToDropItem, Equippable, Equipped, MeleeWeapon, Wearable,
    WantsToRemoveItem, ParticleLifetime, HungerClock, ProvidesFood, MagicMapper, Hidden,
    EntryTrigger, EntityMoved, SingleActivation, BlocksVisibility, Door, Bystander, Vendor,
    Quips, Attributes, Skills, Pools, NaturalAttackDefense, LootTable, Carnivore, Herbivore,
    OtherLevelPosition, LightSource, Initiative, MyTurn, Faction, WantsToApproach,
    WantsToFlee, MoveMode, Chasing,
);