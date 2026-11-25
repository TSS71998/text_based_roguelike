#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct XpColor {
    pub r: u8,
    pub g: u8,
    pub b: u8
}

impl XpColor {
    pub fn is_transparent(&self) -> bool {
        (self.r, self.g, self.b) == (255, 0, 255)
    }

    pub fn to_rgb(self) -> super::RGB {
        super::RGB::from_u8(self.r, self.g, self.b)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct XpCell {
    pub ch: u32,
    pub fg: XpColor,
    pub bg: XpColor
}

pub struct XpLayer {
    pub width: usize,
    pub height: usize,
    cells: Vec<XpCell>
}

impl XpLayer {
    pub fn get(&self, x: usize, y: usize) -> Option<&XpCell> {
        if x < self.width && y < self.height { self.cells.get(x * self.height + y) } else { None }
    }
}

pub struct XpFile {
    pub version: i32,
    pub layers: Vec<XpLayer>
}

fn gunzip(data: &[u8]) -> Result<Vec<u8>, String> {
    if data.len() < 18 || data[0] != 0x1f || data[1] != 0x8b || data[2] != 8 {
        return Err("not gzip data".into())
    }
    let flags = data[3];
    let mut pos = 10;
    let skip_zstr = |pos: &mut usize| -> Result<(), String> {
        let end = data[*pos..].iter().position(|&b| b == 0).ok_or("bad gzip header")?;
        *pos += end + 1;
        Ok(())
    };
    if flags & 4 != 0 {
        let xlen = u16::from_le_bytes([data[pos], data[pos + 1]]) as usize;
        pos += 2 + xlen;
    }
    if flags & 8 != 0 { skip_zstr(&mut pos)?; }
    if flags & 16 != 0 { skip_zstr(&mut pos)?; }
    if flags & 2 != 0 { pos += 2; }
    let body = data.get(pos..data.len().saturating_sub(8)).ok_or("bad gzip header")?;
    miniz_oxide::inflate::decompress_to_vec(body).map_err(|e| format!("gzip data: {:?}", e.status))
}

impl XpFile {
    pub fn from_bytes(data: &[u8]) -> Result<XpFile, String> {
        let raw = gunzip(data)?;
        let mut pos = 0;
        let i32_at = |pos: &mut usize| -> Result<i32, String> {
            let b = raw.get(*pos..*pos + 4).ok_or("truncated .xp file")?;
            *pos += 4;
            Ok(i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        };
        let version = i32_at(&mut pos)?;
        let n_layers = i32_at(&mut pos)?;
        let mut layers = Vec::new();
        for _ in 0..n_layers.max(0) {
            let width = i32_at(&mut pos)?.max(0) as usize;
            let height = i32_at(&mut pos)?.max(0) as usize;
            let mut cells = Vec::with_capacity(width * height);
            for _ in 0..width * height {
                let ch = i32_at(&mut pos)? as u32;
                let c = raw.get(pos..pos+6).ok_or("truncated .xp file")?;
                pos += 6;
                cells.push(XpCell {
                    ch,
                    fg: XpColor { r: c[0], g: c[1], b: c[2] },
                    bg: XpColor { r: c[3], g: c[4], b: c[5] }
                });
            }
            layers.push(XpLayer { width, height, cells });
        }
        Ok(XpFile { version, layers })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
 
    #[test]
    fn loads_menu_background() {
        let xp = XpFile::from_bytes(include_bytes!("../../resources/SmallDungeon_80x50.xp")).unwrap();
        assert!(!xp.layers.is_empty());
        let l = &xp.layers[0];
        assert_eq!((l.width, l.height), (80, 50));
        assert!(l.get(79, 49).is_some() && l.get(80, 0).is_none());
        assert!((0..80).any(|x| l.get(x, 10).unwrap().ch != 32));
    }
}