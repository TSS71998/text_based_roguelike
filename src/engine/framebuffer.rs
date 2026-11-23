use super::color::{RGB, RGBA};

#[inline]
pub fn pack(c: RGB) -> u32 {
    let ch = |v: f32| (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u32;
    (ch(c.r) << 16) | (ch(c.g) << 8) | ch(c.b)
}

#[inline]
pub fn unpack(p: u32) -> RGB {
    RGB::from_u8((p >> 16) as u8, (p >> 8) as u8, p as u8)
}

#[inline]
pub fn blend(dst: u32, src: u32, a: f32) -> u32 {
    if a >= 1.0 {
        return src;
    }
    if a <= 0.0 {
        return dst;
    }
    let a = (a * 256.0) as u32;
    let inv = 256 - a;
    let rb = (((src & 0xFF00FF) * a + (dst & 0xFF00FF) * inv) >> 8) & 0xFF00FF;
    let g = (((src & 0x00FF00) * a + (dst & 0x00FF00) * inv) >> 8) & 0x00FF00;
    rb | g
}

#[derive(Clone)]
pub struct Framebuffer {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u32>
}

impl Framebuffer {
    pub fn new(width: usize, height: usize) -> Framebuffer {
        Framebuffer { width, height, pixels: vec![0; width * height] }
    }

    pub fn clear(&mut self, color: u32) {
        self.pixels.fill(color);
    }

    #[inline]
    pub fn put(&mut self, x: i32, y: i32, color: u32) {
        if x >= 0 && y >= 0 && (x as usize) < self.width && (y as usize) < self.height {
            self.pixels[y as usize * self.width + x as usize] = color;
        }
    }

    #[inline]
    pub fn get(&self, x: i32, y: i32) -> u32 {
        if x >= 0 && y >= 0 && (x as usize) < self.width && (y as usize) < self.height {
            self.pixels[ y as usize * self.width + x as usize]
        } else {
            0
        }
    }

    fn clip(&self, x: i32, y: i32, w: i32, h: i32) -> Option<(usize, usize, usize, usize)> {
        let (x0, y0) = (x.max(0), y.max(0));
        let (x1, y1) = ((x + w).min(self.width as i32), (y + h).min(self.height as i32));
        (x1 > x0 && y1 > y0).then_some((x0 as usize, y0 as usize, x1 as usize, y1 as usize))
    }

    pub fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: u32) {
        if let Some((x0, y0, x1, y1)) = self.clip(x, y, w, h) {
            for row in y0..y1 {
                self.pixels[row * self.width + x0..row * self.width + x1].fill(color);
            }
        }
    }

    pub fn blend_rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: RGBA) {
        if color.a >= 1.0 {
            return self.fill_rect(x, y, w, h, pack(color.to_rgb()));
        }
        if color.a <= 0.0 {
            return;
        }
        let src = pack(color.to_rgb());
        if let Some((x0, y0, x1, y1)) = self.clip(x, y, w, h) {
            for row in y0..y1 {
                for p in &mut self.pixels[row * self.width + x0..row * self.width + x1] {
                    *p = blend(*p, src, color.a);
                }
            }
        }
    }

    pub fn blit_scaled(&mut self, src: &Framebuffer, x: i32, y: i32, w: i32, h: i32) {
        if src.width == 0 || src.height == 0 {
            return;
        }
        if let Some((x0, y0, x1, y1)) = self.clip(x, y, w, h) {
            for row in y0..y1 {
                let sy = ((row as i32 - y) as usize * src.height) / h as usize;
                let src_row = &src.pixels[sy * src.width..(sy + 1) * src.width];
                let dst_row = &mut self.pixels[row * self.width..(row + 1) * self.width];
                for col in x0..x1 {
                    dst_row[col] = src_row[((col as i32 - x) as usize * src.width) / w as usize];
                }
            }
        }
    }

    pub fn crt(&mut self, burn: &[u32], skip: Option<(i32, i32, i32, i32)>) {
        const DARK: u32 = 26;
        let (sx0, sy0, sx1, sy1) = skip.map_or((0, 0, 0, 0), |(x, y, w, h)| {
            (x.max(0) as usize, y.max(0) as usize, (x + w).max(0) as usize, (y + h).max(0) as usize)
        });
        for row in 0..self.height {
            let sub = if row % 2 == 0 { 96 } else { 32 };
            let line = row * self.width..(row + 1) * self.width;
            let skip_row = row >= sy0 && row < sy1;
            for (col, (p, &b)) in self.pixels[line.clone()].iter_mut().zip(&burn[line]).enumerate() {
                if skip_row && col >= sx0 && col < sx1 {
                    continue;
                }
                let (r, g, bl) = ((*p >> 16) & 0xFF, (*p >> 8) & 0xFF, *p & 0xFF);
                *p = if r < DARK && g < DARK && bl < DARK {
                    b
                } else {
                    (r.saturating_sub(sub) << 16) | (g.saturating_sub(sub) << 8) | bl.saturating_sub(sub)
                };
            }
        }
    }

    pub fn screen_burn(width: usize, height: usize) -> Vec<u32> {
        let mut out = Vec::with_capacity(width * height);
        for y in 0..height {
            for x in 0..width {
                let (u, v) = ((x as f32 + 0.5) / width as f32 - 0.5, (y as f32 + 0.5) / height as f32 - 0.5);
                let d = ((1.0 - (u * u + v * v).sqrt()) * 0.2 * 255.0) as u32;
                out.push((d << 8) | d);
            }
        }
        out
    }

    pub fn write_rgba(&self, out: &mut Vec<u8>) {
        out.clear();
        out.reserve(self.pixels.len() * 4);
        for &p in &self.pixels {
            out.extend_from_slice(&[(p >> 16) as u8, (p >> 8) as u8, p as u8, 255]);
        }
    }

    pub fn to_png(&self) -> Vec<u8> {
        let mut rgba = Vec::new();
        self.write_rgba(&mut rgba);
        super::png::encode(self.width, self.height, &rgba)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
 
    #[test]
    fn blend_and_pack() {
        assert_eq!(pack(RGB::from_f32(1.0, 0.5, 0.0)), 0xFF8000);
        assert_eq!(blend(0x000000, 0xFFFFFF, 1.0), 0xFFFFFF);
        assert_eq!(blend(0x123456, 0xFFFFFF, 0.0), 0x123456);
        let half = blend(0x000000, 0xFF00FF, 0.5);
        assert_eq!(half, 0x7F007F);
        assert_eq!(unpack(0xFF8000), RGB::from_u8(255, 128, 0));
    }
 
    #[test]
    fn rects_clip_and_scale() {
        let mut fb = Framebuffer::new(4, 4);
        fb.fill_rect(-2, -2, 4, 4, 0xFF);
        assert_eq!(fb.get(1, 1), 0xFF);
        assert_eq!(fb.get(2, 2), 0);
        let mut src = Framebuffer::new(2, 1);
        src.pixels = vec![1, 2];
        let mut dst = Framebuffer::new(4, 2);
        dst.blit_scaled(&src, 0, 0, 4, 2);
        assert_eq!(dst.pixels, vec![1, 1, 2, 2, 1, 1, 2, 2]);
    }
}