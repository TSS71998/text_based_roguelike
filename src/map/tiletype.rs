use serde::{Serialize, Deserialize};
use specs::rayon::iter::{IntoParallelRefIterator, ParallelIterator};

#[derive(PartialEq, Copy, Clone, Serialize, Deserialize, Eq, Hash)]
pub enum TileType {
    Wall, 
    Stalactite, 
    Stalagmite,
    Floor, 
    DownStairs,
    Road,
    Grass,
    ShallowWater,
    DeepWater,
    WoodFloor,
    Bridge,
    Gravel,
    UpStairs
}

pub fn tile_walkable(tt: TileType) -> bool {
    match tt {
        TileType::Floor | TileType::DownStairs | TileType::Road | TileType::Grass |
        TileType::ShallowWater | TileType::WoodFloor | TileType::Bridge | TileType::Gravel | 
        TileType::UpStairs
            => true,
          _ => false
    }
}

pub fn tile_opaque(tt: TileType) -> bool {
    match tt {
        TileType::Wall | TileType::Stalactite | TileType::Stalagmite => true,
        _ => false
    }
}

pub fn tile_cost(tt: TileType) -> f32 {
    match tt {
        TileType::Road => 0.8,
        TileType::Grass => 1.1,
        TileType::ShallowWater => 1.2,
        _ => 1.0
    }
}

#[derive(Default, Serialize, Clone, Deserialize)]
pub struct TileEncoder(pub Vec<u8>);

impl TileEncoder {
    pub fn encode(tiles: &Vec<TileType>) -> Self {
        TileEncoder(tiles.par_iter().copied().map(|tile| match tile {
            TileType::Wall => 0,
            TileType::Floor => 1,
            TileType::DownStairs => 2,
            TileType::Road => 3,
            TileType::Grass => 4,
            TileType::ShallowWater => 5,
            TileType::DeepWater => 6,
            TileType::WoodFloor => 7,
            TileType::Bridge => 8,
            TileType::Gravel => 9,
            TileType::UpStairs => 10,
            TileType::Stalactite => 11,
            TileType::Stalagmite => 12
        }).collect())
    }

    pub fn decode(self) -> Vec<TileType> {
        self.0.into_iter().map(|tile| match tile {
            0 => TileType::Wall,
            1 => TileType::Floor,
            2 => TileType::DownStairs,
            3 => TileType::Road,
            4 => TileType::Grass,
            5 => TileType::ShallowWater,
            6 => TileType::DeepWater,
            7 => TileType::WoodFloor,
            8 => TileType::Bridge,
            9 => TileType::Gravel,
            10 => TileType::UpStairs,
            11 => TileType::Stalactite,
            12 => TileType::Stalagmite,
            _ => unimplemented!()
        }).collect()
    }
}