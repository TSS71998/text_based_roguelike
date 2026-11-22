//! Binary save format.
//!
//! Layout of `savegame.sav`:
//!
//! ```text
//! offset  size  field
//! 0       4     magic  b"RLSV"
//! 4       2     format version (u16, little endian)
//! 6       1     flags  (bit 0 = payload is zlib-compressed)
//! 7       4     uncompressed payload length (u32, little endian)
//! 11      ..    payload
//! ```

use std::collections::HashMap;
use std::convert::Infallible;
use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::path::Path;

use bincode::Options;
use rltk::Point;
use specs::prelude::*;
use specs::saveload::{DeserializeComponents, SerializeComponents, SimpleMarker, SimpleMarkerAllocator};

use super::components::*;
use super::map::{Map, MasterDungeonMap};

mod codec;
use codec::{ByteReader, ByteWriter, Corrupt};

const SAVE_PATH: &str = "./savegame.sav";
const MAGIC: &[u8; 4] = b"RLSV";
const SAVE_VERSION: u16 = 2;
const HEADER_LEN: usize = 11;
const FLAG_ZLIB: u8 = 1;
const COMPRESSION_LEVEL: u8 = 6;
const MAX_PAYLOAD: usize = 256 * 1024 * 1024;

#[derive(Debug)]
pub enum SaveError{
    Io(io::Error),
    Format(String),
    UnsupportedVersion {found: u16, expected: u16},
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

impl From<bincode::Error> for SaveError {
    fn from(e: bincode::Error) -> Self { SaveError::Format(e.to_string()) }
}

impl From<Corrupt> for SaveError {
    fn from(e: Corrupt) -> Self { SaveError::Format(e.0.to_string()) }
}

fn bincode_options() -> impl Options + Copy {
    bincode::DefaultOptions::new()
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
    let bytes = encode_save(ecs)?;
    let tmp = path.with_extension("sav.tmp");
    fs::write(&tmp, &bytes)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

fn encode_save(ecs: &World) -> Result<Vec<u8>, SaveError> {
    let mut w = ByteWriter::with_capacity(64 * 1024);
    {
        let map = ecs.fetch::<Map>();
        codec::write_map(&mut w, &map);

        let dungeon = ecs.fetch::<MasterDungeonMap>();
        let stored = dungeon.maps_by_depth();
        w.varint(stored.len() as u64);
        for m in stored {
            codec::write_map(&mut w, m);
        }
    }
    serialize_components(ecs, &mut w)?;

    let payload = w.buf;
    let payload_len = u32::try_from(payload.len())
        .map_err(|_| SaveError::Format("save too large".into()))?;
    let body = miniz_oxide::deflate::compress_to_vec_zlib(&payload, COMPRESSION_LEVEL);

    let mut out = Vec::with_capacity(HEADER_LEN + body.len());
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&SAVE_VERSION.to_le_bytes());
    out.push(FLAG_ZLIB);
    out.extend_from_slice(&payload_len.to_le_bytes());
    out.extend_from_slice(&body);
    Ok(out)
}

pub fn load_game_from(ecs: &mut World, path: impl AsRef<Path>) -> Result<(), SaveError> {
    let bytes = fs::read(path)?;
    let payload = decode_container(&bytes)?;
    drop(bytes);

    let mut r = ByteReader::new(&payload);
    let map = codec::read_map(&mut r)?;
    let stored_count = r.usize()?;
    let mut stored = Vec::with_capacity(stored_count.min(256));
    for _ in 0..stored_count {
        stored.push(codec::read_map(&mut r)?);
    }
    let component_count = r.usize()?;
    let mut blobs: HashMap<String, &[u8]> = HashMap::with_capacity(component_count.min(256));
    let mut player_count = 0;
    for _ in 0..component_count {
        let name = r.str()?;
        let entities = r.varint()?;
        let blob = r.bytes()?;
        if name == "Player" {
            player_count = entities;
        }
        blobs.insert(name, blob);
    }
    if !r.is_empty() {
        return Err(Corrupt("trailing data").into());
    }
    if player_count == 0 {
        return Err(SaveError::MissingPlayer);
    }

    ecs.delete_all();
    ecs.maintain();
    ecs.insert(SimpleMarkerAllocator::<SerializeMe>::new());
    deserialize_components(ecs, &blobs)?;

    crate::spatial::set_size(map.tiles.len());
    *ecs.write_resource::<Map>() = map;
    *ecs.write_resource::<MasterDungeonMap>() = MasterDungeonMap::from_maps(stored);

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

fn decode_container(bytes: &[u8]) -> Result<Vec<u8>, SaveError> {
    if bytes.len() < HEADER_LEN || &bytes[0..4] != MAGIC {
        return Err(SaveError::Format("not a save file".into()));
    }
    let version = u16::from_le_bytes([bytes[4], bytes[5]]);
    if version != SAVE_VERSION {
        return Err(SaveError::UnsupportedVersion { found: version, expected: SAVE_VERSION });
    }
    let flags = bytes[6];
    let len = u32::from_le_bytes([bytes[7], bytes[8], bytes[9], bytes[10]]) as usize;
    if len > MAX_PAYLOAD {
        return Err(SaveError::Format("payload too large".into()));
    }
    let body = &bytes[HEADER_LEN..];

    let payload = if flags & FLAG_ZLIB != 0 {
        miniz_oxide::inflate::decompress_to_vec_zlib_with_limit(body, len)
            .map_err(|e| SaveError::Format(format!("decompression failed: {:?}", e.status)))?
    } else {
        body.to_vec()
    };
    if payload.len() != len {
        return Err(SaveError::Format("payload length mismatch".into()));
    }
    Ok(payload)
}

macro_rules! savable_components {
    ($($ty:ident),+ $(,)?) => {
        fn register_savable_components(ecs: &mut World) {
            ecs.register::<SimpleMarker<SerializeMe>>();
            $(ecs.register::<$ty>();)+
        }

        fn serialize_components(ecs: &World, w: &mut ByteWriter) -> Result<(), SaveError> {
            let entities = ecs.entities();
            let markers = ecs.read_storage::<SimpleMarker<SerializeMe>>();
            let opts = bincode_options();
            let mut blob = Vec::new();

            w.varint([$(stringify!($ty)),+].len() as u64);
            $(
                let storage = ecs.read_storage::<$ty>();
                let count = (&storage, &markers).join().count();
                blob.clear();
                SerializeComponents::<Infallible, SimpleMarker<SerializeMe>>::serialize(
                    &(storage,),
                    &entities,
                    &markers,
                    &mut bincode::Serializer::new(&mut blob, opts),
                )?;
                w.str(stringify!($ty));
                w.varint(count as u64);
                w.bytes(&blob);
            )+
            Ok(())
        }

        fn deserialize_components(ecs: &World, blobs: &HashMap<String, &[u8]>) -> Result<(), SaveError> {
            let entities = ecs.entities();
            let mut markers = ecs.write_storage::<SimpleMarker<SerializeMe>>();
            let mut allocator = ecs.write_resource::<SimpleMarkerAllocator<SerializeMe>>();
            let opts = bincode_options();
            $(
                // A component absent from the file (e.g. added after the save was made) is skipped.
                if let Some(blob) = blobs.get(stringify!($ty)) {
                    DeserializeComponents::<Infallible, _>::deserialize(
                        &mut (&mut ecs.write_storage::<$ty>(),),
                        &entities,
                        &mut markers,
                        &mut allocator,
                        &mut bincode::Deserializer::from_slice(blob, opts),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{gamelog, lighting_system, map, particle_system, raws, rex_assets, spawner};
    use specs::saveload::Marker;
    use std::time::Instant;

    /// Map generation uses the global spatial index, so these tests can't run in parallel.
    fn serial() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// A headless world that has visited five levels, with most tiles revealed.
    fn played_world() -> World {
        let mut ecs = World::new();
        register_components(&mut ecs);
        ecs.register::<Attribute>();
        ecs.register::<EquipmentChanged>();
        ecs.insert(SimpleMarkerAllocator::<SerializeMe>::new());
        raws::load_raws();
        ecs.insert(MasterDungeonMap::new());
        ecs.insert(Map::new(1, 64, 64, "New Map"));
        ecs.insert(Point::new(0, 0));
        ecs.insert(rltk::RandomNumberGenerator::seeded(42));
        let player = spawner::player(&mut ecs, 0, 0);
        ecs.insert(player);
        ecs.insert(gamelog::GameLog { entries: vec![] });
        ecs.insert(particle_system::ParticleBuilder::new());
        ecs.insert(lighting_system::LightingState::default());
        ecs.insert(rex_assets::RexAssets::new());
        map::level_transition(&mut ecs, 1, 0);
        for depth in 2..=5 {
            map::freeze_level_entities(&mut ecs);
            if map::level_transition(&mut ecs, depth, 1).is_none() {
                map::thaw_level_entities(&ecs);
            }
            let mut m = ecs.write_resource::<Map>();
            let n = m.tiles.len();
            for i in 0..n {
                if (i / 7) % 5 != 0 { m.revealed_tiles[i] = true; }
            }
            for i in (0..n).step_by(97) { m.bloodstains[i] = true; }
            for i in 300..340 { m.visible_tiles[i] = true; }
        }
        ecs.maintain();
        ecs
    }

    fn fresh_world() -> World {
        let mut ecs = World::new();
        register_components(&mut ecs);
        ecs.insert(SimpleMarkerAllocator::<SerializeMe>::new());
        ecs.insert(MasterDungeonMap::new());
        ecs.insert(Map::new(1, 10, 10, "Empty"));
        ecs.insert(Point::new(0, 0));
        let placeholder = ecs.create_entity().build();
        ecs.insert(placeholder);
        ecs
    }

    /// Splits a payload into ("maps", bytes) followed by one entry per component.
    fn sections(payload: &[u8]) -> Vec<(String, Vec<u8>)> {
        let mut r = ByteReader::new(payload);
        codec::read_map(&mut r).unwrap();
        for _ in 0..r.usize().unwrap() { codec::read_map(&mut r).unwrap(); }
        let mut out = vec![("maps".to_string(), payload[..payload.len() - r.remaining()].to_vec())];
        for _ in 0..r.usize().unwrap() {
            let name = r.str().unwrap();
            r.varint().unwrap();
            out.push((name, r.bytes().unwrap().to_vec()));
        }
        out
    }

    fn skills_by_marker(ecs: &World) -> HashMap<u64, HashMap<Skill, i32>> {
        let markers = ecs.read_storage::<SimpleMarker<SerializeMe>>();
        let skills = ecs.read_storage::<Skills>();
        (&markers, &skills).join().map(|(m, s)| (m.id(), s.skills.clone())).collect()
    }

    #[test]
    fn round_trip_is_lossless_and_small() {
        let _guard = serial();
        let src = played_world();
        let dir = std::env::temp_dir().join(format!("rl_save_test_{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("a.sav");

        let t = Instant::now();
        save_game_to(&src, &path).unwrap();
        let save_time = t.elapsed();
        let size = fs::metadata(&path).unwrap().len();

        let mut dst = fresh_world();
        let t = Instant::now();
        load_game_from(&mut dst, &path).unwrap();
        let load_time = t.elapsed();
        println!("save file: {size} bytes, save {save_time:?}, load {load_time:?}");
        assert!(size < 200 * 1024, "save is {size} bytes");

        // Re-saving the loaded world must reproduce the same bytes. `Skills` holds a
        // HashMap whose iteration order is random, so it is compared by value instead.
        let a = decode_container(&encode_save(&src).unwrap()).unwrap();
        let b = decode_container(&encode_save(&dst).unwrap()).unwrap();
        let (sa, sb) = (sections(&a), sections(&b));
        assert_eq!(sa.len(), sb.len());
        for ((name_a, blob_a), (name_b, blob_b)) in sa.iter().zip(&sb) {
            assert_eq!(name_a, name_b);
            if name_a != "Skills" {
                assert!(blob_a == blob_b, "{name_a} differs after round trip");
            }
        }
        assert_eq!(skills_by_marker(&src), skills_by_marker(&dst));

        let (m1, m2) = (src.fetch::<Map>(), dst.fetch::<Map>());
        assert!(m1.tiles == m2.tiles && m1.revealed_tiles == m2.revealed_tiles
            && m1.visible_tiles == m2.visible_tiles && m1.bloodstains == m2.bloodstains);
        assert_eq!((m1.width, m1.height, m1.depth, &m1.name, m1.outdoors),
                   (m2.width, m2.height, m2.depth, &m2.name, m2.outdoors));
        assert_eq!(m2.light.len(), m2.tiles.len());
        assert_eq!(m2.view_blocked.len(), m2.tiles.len());
        assert_eq!(*src.fetch::<Point>(), *dst.fetch::<Point>());
        assert_eq!(dst.read_storage::<Player>().join().count(), 1);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn corrupt_save_is_rejected_without_touching_world() {
        let _guard = serial();
        let src = played_world();
        let good = encode_save(&src).unwrap();
        let mut dst = fresh_world();
        let before = dst.entities().join().count();
        let dir = std::env::temp_dir().join(format!("rl_corrupt_test_{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("bad.sav");

        let mut flipped = good.clone();
        let mid = flipped.len() / 2;
        flipped[mid] ^= 0x5a;
        let mut bad_version = good.clone();
        bad_version[4] = 99;
        for bad in [flipped, bad_version, good[..good.len() / 2].to_vec(), b"{\"version\":1}".to_vec()] {
            fs::write(&path, &bad).unwrap();
            assert!(load_game_from(&mut dst, &path).is_err());
            assert_eq!(dst.entities().join().count(), before);
        }
        fs::remove_dir_all(&dir).ok();
    }
}