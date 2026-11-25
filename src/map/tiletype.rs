use serde::{Deserialize, Serialize};

#[derive(PartialEq, Copy, Clone, Debug, Serialize, Deserialize, Eq, Hash)]
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
    UpStairs,
}

#[derive(PartialEq, Copy, Clone, Debug, Eq)]
pub enum TileShape {
    Solid,
    Column,
    Floor,
    Liquid,
    StairsDown,
    StairsUp,
}

#[derive(Copy, Clone, Debug)]
pub struct TileDef {
    pub walkable: bool,
    pub opaque: bool,
    pub cost: f32,
    pub shape: TileShape,
}

const fn def(walkable: bool, opaque: bool, cost: f32, shape: TileShape) -> TileDef {
    TileDef {
        walkable,
        opaque,
        cost,
        shape,
    }
}

pub const fn tile_def(tt: TileType) -> TileDef {
    use TileShape::*;
    match tt {
        TileType::Wall => def(false, true, 1.0, Solid),
        TileType::Stalactite => def(false, true, 1.0, Column),
        TileType::Stalagmite => def(false, true, 1.0, Column),
        TileType::Floor => def(true, false, 1.0, Floor),
        TileType::DownStairs => def(true, false, 1.0, StairsDown),
        TileType::Road => def(true, false, 0.8, Floor),
        TileType::Grass => def(true, false, 1.1, Floor),
        TileType::ShallowWater => def(true, false, 1.2, Liquid),
        TileType::DeepWater => def(false, false, 1.0, Liquid),
        TileType::WoodFloor => def(true, false, 1.0, Floor),
        TileType::Bridge => def(true, false, 1.0, Floor),
        TileType::Gravel => def(true, false, 1.0, Floor),
        TileType::UpStairs => def(true, false, 1.0, StairsUp),
    }
}

#[inline]
pub fn tile_walkable(tt: TileType) -> bool {
    tile_def(tt).walkable
}

#[inline]
pub fn tile_opaque(tt: TileType) -> bool {
    tile_def(tt).opaque
}

#[inline]
pub fn tile_cost(tt: TileType) -> f32 {
    tile_def(tt).cost
}
