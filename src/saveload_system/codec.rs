use crate::map::{Map, TileType};

const MAX_TILES: usize = 1 << 24;

const TILE_IDS: [TileType; 13] = [
    TileType::Wall,
    TileType::Stalactite,
    TileType::Stalagmite,
    TileType::Floor,
    TileType::DownStairs,
    TileType::Road,
    TileType::Grass,
    TileType::ShallowWater,
    TileType::DeepWater,
    TileType::WoodFloor,
    TileType::Bridge,
    TileType::Gravel,
    TileType::UpStairs,
];

fn tile_to_id(t: TileType) -> u8 {
    match t {
        TileType::Wall => 0,
        TileType::Stalactite => 1,
        TileType::Stalagmite => 2,
        TileType::Floor => 3,
        TileType::DownStairs => 4,
        TileType::Road => 5,
        TileType::Grass => 6,
        TileType::ShallowWater => 7,
        TileType::DeepWater => 8,
        TileType::WoodFloor => 9,
        TileType::Bridge => 10,
        TileType::Gravel => 11,
        TileType::UpStairs => 12,
    }
}

#[derive(Debug)]
pub struct Corrupt(pub &'static str);

pub type DecodeResult<T> = Result<T, Corrupt>;

#[derive(Default)]
pub struct ByteWriter {
    pub buf: Vec<u8>
}

impl ByteWriter {
    pub fn with_capacity(n: usize) -> Self {
        ByteWriter { buf: Vec::with_capacity(n) }
    }

    pub fn u8(&mut self, v: u8) {
        self.buf.push(v);
    }

    pub fn bool(&mut self, v: bool) {
        self.buf.push(v as u8);
    }

    /// Unsigned LEB128: 7 bits per byte, high bit = "more bytes follow".
    pub fn varint(&mut self, mut v: u64) {
        while v >= 0x80 {
            self.buf.push((v as u8) | 0x80);
            v >>= 7;
        }
        self.buf.push(v as u8);
    }

    pub fn varint_i(&mut self, v: i64) {
        self.varint(((v << 1) ^ (v >> 63)) as u64);
    }

    pub fn bytes(&mut self, b: &[u8]) {
        self.varint(b.len() as u64);
        self.buf.extend_from_slice(b);
    }

    pub fn str(&mut self, s: &str) {
        self.bytes(s.as_bytes());
    }

    pub fn tiles(&mut self, tiles: &[TileType]) {
        let mut i = 0;
        while i < tiles.len() {
            let t = tiles[i];
            let start = i;
            while i < tiles.len() && tiles[i] == t {
                i += 1;
            }
            self.u8(tile_to_id(t));
            self.varint((i - start) as u64);
        }
    }

    pub fn bits(&mut self, bits: &[bool]) {
        let mut current = false;
        let mut i = 0;
        while i < bits.len() {
            let start = i;
            while i < bits.len() && bits[i] == current {
                i += 1;
            }
            self.varint((i - start) as u64);
            current = !current;
        }
    }
}

pub struct ByteReader<'a> {
    data: &'a [u8],
    pos: usize
}

impl<'a> ByteReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        ByteReader { data, pos: 0 }
    }

    #[cfg(test)]
    pub fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    pub fn is_empty(&self) -> bool {
        self.pos >= self.data.len()
    }

    pub fn take(&mut self, n: usize) -> DecodeResult<&'a [u8]> {
        let end = self.pos.checked_add(n).ok_or(Corrupt("length overflow"))?;
        let s = self.data.get(self.pos..end).ok_or(Corrupt("unxpected end of data"))?;
        self.pos = end;
        Ok(s)
    }

    pub fn u8(&mut self) -> DecodeResult<u8> {
        Ok(self.take(1)?[0])
    }

    pub fn bool(&mut self) -> DecodeResult<bool> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(Corrupt("invalid bool"))
        }
    }

    pub fn varint(&mut self) -> DecodeResult<u64> {
        let mut v: u64 = 0;
        for shift in (0..64).step_by(7) {
            let b = self.u8()?;
            v |= ((b & 0x7f) as u64) << shift;
            if b & 0x80 == 0 {
                return Ok(v);
            }
        }
        Err(Corrupt("varint too long"))
    }

    pub fn varint_i(&mut self) -> DecodeResult<i64> {
        let v = self.varint()?;
        Ok(((v >> 1) as i64) ^ ((v & 1) as i64))
    }

    pub fn usize(&mut self) -> DecodeResult<usize> {
        usize::try_from(self.varint()?).map_err(|_| Corrupt("length too large"))
    }

    pub fn i32(&mut self) -> DecodeResult<i32> {
        i32::try_from(self.varint_i()?).map_err(|_| Corrupt("integer out of range"))
    }

    pub fn bytes(&mut self) -> DecodeResult<&'a [u8]> {
        let n = self.usize()?;
        self.take(n)
    }

    pub fn str(&mut self) -> DecodeResult<String> {
        let b =self.bytes()?;
        String::from_utf8(b.to_vec()).map_err(|_| Corrupt("invalid utf-8"))
    }

    pub fn tiles(&mut self, count: usize) -> DecodeResult<Vec<TileType>> {
        let mut out = Vec::with_capacity(count);
        while out.len() < count {
            let id = self.u8()? as usize;
            let tile = *TILE_IDS.get(id).ok_or(Corrupt("unknown tile type"))?;
            let run = self.usize()?;
            if run == 0 || run > count - out.len() {
                return Err(Corrupt("bad tile run"));
            }
            out.resize(out.len() + run, tile);
        }
        Ok(out)
    }

    pub fn bits(&mut self, count: usize) -> DecodeResult<Vec<bool>> {
        let mut out = Vec::with_capacity(count);
        let mut current = false;
        while out.len() < count {
            let run = self.usize()?;
            if run > count - out.len() {
                return Err(Corrupt("bad bit run"));
            }
            out.resize(out.len() + run, current);
            current = !current;
        }
        Ok(out)
    }
}

pub fn write_map(w: &mut ByteWriter, map: &Map) {
    w.varint_i(map.width as i64);
    w.varint_i(map.height as i64);
    w.varint_i(map.depth as i64);
    w.str(&map.name);
    w.bool(map.outdoors);
    w.tiles(&map.tiles);
    w.bits(&map.revealed_tiles);
    w.bits(&map.visible_tiles);
    w.bits(&map.bloodstains);
}

pub fn read_map(r: &mut ByteReader) -> DecodeResult<Map> {
    let width = r.i32()?;
    let height = r.i32()?;
    if width <= 0 || height <= 0  {
        return Err(Corrupt("bad map dimensions"));
    }
    let count = (width as usize)
        .checked_mul(height as usize)
        .filter(|&n| n <= MAX_TILES)
        .ok_or(Corrupt("map too large"))?;
    let depth = r.i32()?;
    let name = r.str()?;
    let outdoors = r.bool()?;
    let tiles = r.tiles(count)?;
    let revealed_tiles = r.bits(count)?;
    let visible_tiles = r.bits(count)?;
    let bloodstains = r.bits(count)?;

    Ok(Map {
        tiles,
        width,
        height,
        revealed_tiles,
        visible_tiles,
        depth,
        bloodstains,
        view_blocked: vec![false; count],
        name,
        outdoors,
        light: vec![rltk::RGB::from_f32(0.0, 0.0, 0.0); count],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn varints_round_trip() {
        let mut w = ByteWriter::default();
        let vals = [0u64, 1, 127, 128, 300, u32::MAX as u64, u64::MAX];
        let ints = [0i64, -1, 1, -64, 64, i32::MIN as i64, i64::MAX, i64::MIN];
        vals.iter().for_each(|&v| w.varint(v));
        ints.iter().for_each(|&v| w.varint_i(v));
        let mut r = ByteReader::new(&w.buf);
        for &v in &vals { assert_eq!(r.varint().unwrap(), v); }
        for &v in &ints { assert_eq!(r.varint_i().unwrap(), v); }
        assert!(r.is_empty());
    }

    #[test]
    fn tile_ids_match_table() {
        for (i, t) in TILE_IDS.iter().enumerate() {
            assert_eq!(tile_to_id(*t) as usize, i);
        }
    }

    #[test]
    fn bits_round_trip() {
        for bits in [vec![], vec![true], vec![false; 10], vec![true, true, false, true, false, false]] {
            let mut w = ByteWriter::default();
            w.bits(&bits);
            assert_eq!(ByteReader::new(&w.buf).bits(bits.len()).unwrap(), bits);
        }
    }

    #[test]
    fn truncated_input_is_an_error() {
        let mut map = Map::new(3, 20, 10, "Test");
        map.tiles[5] = TileType::Floor;
        map.revealed_tiles[7] = true;
        let mut w = ByteWriter::default();
        write_map(&mut w, &map);
        for cut in 0..w.buf.len() {
            assert!(read_map(&mut ByteReader::new(&w.buf[..cut])).is_err());
        }
        let back = read_map(&mut ByteReader::new(&w.buf)).unwrap();
        assert!(back.tiles == map.tiles && back.revealed_tiles == map.revealed_tiles);
    }
}