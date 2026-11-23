//! Just enough PNG: decode non-interlaced 8-bit images (and 1/2/4-bit palettes) to RGBA,
//! and encode RGBA for screenshots. Compression is miniz_oxide's zlib.

pub struct Image {
    pub width: usize,
    pub height: usize,
    pub rgba: Vec<u8>
}

const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n'];

fn be32(b: &[u8]) -> u32 {
    u32::from_be_bytes([b[0], b[1], b[2], b[3]])
}

pub fn decode(data: &[u8]) -> Result<Image, String> {
    if data.len() < 8 || data[..8] != SIGNATURE {
        return Err("not a PNG file".into());
    }
    let (mut width, mut height, mut depth, mut color, mut interlace) = (0usize, 0usize, 0u8, 0u8, 0u8);
    let (mut palette, mut trns, mut idat) = (Vec::new(), Vec::new(), Vec::new());
    let mut pos = 8;
    while pos + 12 <= data.len() {
        let len = be32(&data[pos..]) as usize;
        let kind = &data[pos + 4..pos + 8];
        let body = data.get(pos + 8..pos + 8 + len).ok_or("truncated chunk")?;
        match kind {
            b"IHDR" => {
                width = be32(body) as usize;
                height = be32(&body[4..]) as usize;
                depth = body[8];
                color = body[9];
                interlace = body[12];
            }
            b"PLTE" => palette = body.to_vec(),
            b"tRNS" => trns = body.to_vec(),
            b"IDAT" => idat.extend_from_slice(body),
            b"IEND" => break,
            _ => {}
        }
        pos += 12 + len;
    }
    if interlace != 0 {
        return Err("interlaced PNGs are not supported".into());
    }
    let channels = match color {
        0 => 1,
        2 => 3,
        3 => 1,
        4 => 2,
        6 => 4,
        _ => return Err(format!("unsupported PNG colour type {color}")),
    };
    if depth != 8 && !(color == 3 && matches!(depth, 1 | 2 | 4)) {
        return Err(format!("unsupported PNG bit depth {depth}"));
    }
    if width == 0 || height == 0 || width * height > 1 << 26 {
        return Err("bad PNG dimensions".into());
    }

    let raw = miniz_oxide::inflate::decompress_to_vec_zlib(&idat).map_err(|e| format!("PNG data: {:?}", e.to_string()))?;
    let bits_pp = channels * depth as usize;
    let stride = (width * bits_pp).div_ceil(8);
    let bpp = (bits_pp / 8).max(1);
    if raw.len() < height * (stride + 1) {
        return Err("PNG data too short".into());
    }

    let mut pixels = vec![0u8; height * stride];
    for y in 0..height {
        let filter = raw[y * (stride + 1)];
        let src = &raw[y * (stride + 1) + 1..(y + 1) * (stride + 1)];
        let (done, rest) = pixels.split_at_mut(y * stride);
        let prev = if y > 0 { &done[(y - 1) * stride..] } else { &[][..] };
        let row = &mut rest[..stride];
        for x in 0..stride {
            let a = if x >= bpp { row[x - bpp] as i32 } else { 0 };
            let b = prev.get(x).copied().unwrap_or(0) as i32;
            let c = if x >= bpp { prev.get(x - bpp).copied().unwrap_or(0) as i32 } else { 0 };
            let predicted = match filter {
                0 => 0,
                1 => a,
                2 => b,
                3 => (a + b) / 2,
                4 => {
                    let p = a + b - c;
                    let (pa, pb, pc) = ((p - a).abs(), (p - b).abs(), (p - c).abs());
                    if pa <= pb && pa <= pc { a } else if pb <= pc { b } else { c }
                }
                _ => return Err(format!("bad PNG filter {filter}")),
            };
            row[x] = (src[x] as i32 + predicted) as u8; 
        }
    }

    let mut rgba = Vec::with_capacity(width * height * 4);
    for y in 0..height {
        let row = &pixels[y * stride..(y + 1) * stride];
        for x in 0..width {
            let px = match color {
                0 => [row[x], row[x], row[x], 255],
                2 => [row[x * 3], row[x * 3 + 1], row[x * 3 + 2], 255],
                4 => [row[x * 2], row[x * 2], row[x * 2], row[x * 2 + 1]],
                6 => [row[x * 4], row[x * 4 + 1], row[x * 4 + 2], row[x * 4 + 3]],
                _ => {
                    let d = depth as usize;
                    let bit = x * d;
                    let i = ((row[bit / 8] >> (8 - d - bit % 8)) & ((1 << d) - 1) as u8) as usize;
                    let p = palette.get(i * 3..i * 3 + 3).ok_or("palette index out of range")?;
                    [p[0], p[1], p[2], trns.get(i).copied().unwrap_or(255)]
                }
            };
            rgba.extend_from_slice(&px);
        }
    }

    Ok(Image { width, height, rgba })
}

fn crc32(chunks: &[&[u8]]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for chunk in chunks {
        for &b in *chunk {
            crc ^= b as u32;
            for _ in 0..8 {
                crc = if crc & 1 != 0 { (crc >> 1) ^ 0xEDB8_8320 } else { crc>> 1 };
            }
        }
    }
    !crc
}

pub fn encode(width: usize, height: usize, rgba: &[u8]) -> Vec<u8> {
    let mut raw = Vec::with_capacity(height * (width * 4 + 1));
    for row in rgba.chunks_exact(width * 4).take(height) {
        raw.push(0);
        raw.extend_from_slice(row);
    }
    let mut out = SIGNATURE.to_vec();
    let mut chunk = |kind: &[u8; 4], body: &[u8]| {
        out.extend_from_slice(&(body.len() as u32).to_be_bytes());
        out.extend_from_slice(kind);
        out.extend_from_slice(body);
        out.extend_from_slice(&crc32(&[kind, body]).to_be_bytes());
    };
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&(width as u32).to_be_bytes());
    ihdr.extend_from_slice(&(height as u32).to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    chunk(b"IHDR", &ihdr);
    chunk(b"IDAT", &miniz_oxide::deflate::compress_to_vec_zlib(&raw, 6));
    chunk(b"IEND", &[]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
 
    #[test]
    fn encode_then_decode() {
        let (w, h) = (5, 3);
        let rgba: Vec<u8> = (0..w * h * 4).map(|i| (i * 37 % 256) as u8).collect();
        let img = decode(&encode(w, h, &rgba)).unwrap();
        assert_eq!((img.width, img.height), (w, h));
        assert_eq!(img.rgba, rgba);
        assert_eq!(crc32(&[b"IEND"]), 0xAE42_6082);
    }
    
    #[test]
    fn decodes_the_game_font() {
        let img = decode(include_bytes!("../../resources/terminal8x8.png")).unwrap();
        assert_eq!((img.width, img.height), (128, 128));
        // '@' (64) sits at column 0, row 4; some of its pixels are lit.
        let lit = (0..8).flat_map(|y| (0..8).map(move |x| (x, y)))
            .filter(|&(x, y)| img.rgba[((32 + y) * 128 + x) * 4 + 3] > 0 && img.rgba[((32 + y) * 128 + x) * 4] > 128)
            .count();
        assert!(lit > 10, "{lit}");
    }
}