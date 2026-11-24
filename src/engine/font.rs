use super::color::RGBA;
use super::framebuffer::{blend, pack, Framebuffer};

pub struct Font {
    pub glyph_w: usize,
    pub glyph_h: usize,
    masks: Vec<bool>,
    cell_w: usize,
    cell_h: usize,
    cell_masks: Vec<bool>
}

impl Font {
    pub fn from_png(png: &[u8], glyph_w: usize, glyph_h: usize) -> Result<Font, String> {
        let img = super::png::decode(png)?;
        if img.width < glyph_w * 16 || img.height < glyph_h * 16 {
            return Err(format!("font image is {}x{}, expected 16x16 glyphs of {glyph_w}x{glyph_h}", img.width, img.height));
        }
        let mut masks = vec![false; 256 * glyph_w * glyph_h];
        for g in 0..256 {
            let (gx, gy) = ((g % 16) * glyph_w, (g / 16) * glyph_h);
            for y in 0..glyph_h {
                for x in 0..glyph_w {
                    let i = ((gy + y) * img.width + gx + x) * 4;
                    let px = &img.rgba[i..i + 4];
                    let lum = (px[0] as u32 * 299 + px[1] as u32 * 587 + px[2] as u32 * 114) / 1000;
                    masks[(g * glyph_h + y) * glyph_w + x] = lum * px[3] as u32 / 255 >= 128;
                }
            }
        }
        Ok(Font { glyph_w, glyph_h, masks, cell_w: 0, cell_h: 0, cell_masks: Vec::new() })
    }

    pub fn prepare_cell_size(&mut self, cell_w: usize, cell_h: usize) {
        self.cell_w = cell_w;
        self.cell_h = cell_h;
        self.masks = vec![false; 256 * cell_w * cell_h];
        for g in 0..256 {
            for y in 0..cell_h {
                for x in 0..cell_w {
                    let (sx, sy) = (x * self.glyph_w / cell_w, y * self.glyph_h / cell_h);
                    self.cell_masks[(g * cell_h + y) * cell_w + x] = self.masks[(g * self.glyph_h + sy) * self.glyph_w + sx];
                }
            }
        }
    }

    pub fn draw_cell(&self, fb: &mut Framebuffer, px: i32, py: i32, glyph: u16, fg: RGBA, bg: RGBA) {
        let (cw, ch) = (self.cell_w, self.cell_h);
        fb.blend_rect(px, py, cw as i32, ch as i32, bg);
        if fg.a <= 0.0 {
            return;
        }
        let mask = &self.cell_masks[(glyph as usize & 255) * cw * ch..][..cw * ch];
        let color = pack(fg.to_rgb());
        let opaque = fg.a >= 1.0;
        for y in 0..ch {
            let fy = py + y as i32;
            if fy < 0 || fy as usize >= fb.height {
                continue;
            }
            let row = fy as usize * fb.width;
            for x in 0..cw {
                let fx = px + x as i32;
                if mask[y * cw + x] && fx >= 0 && (fx as usize) < fb.width {
                    let p = &mut fb.pixels[row + fx as usize];
                    *p = if opaque { color } else { blend(*p, color, fg.a) };
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw_scaled(&self, fb: &mut Framebuffer, x: f32, y: f32, w: f32, h: f32, glyph: u16, fg: RGBA, bg: RGBA) {
        let (x0, y0) = (x.round() as i32, y.round() as i32);
        let (x1, y1) = ((x + w).round() as i32, (y + h).round() as i32);
        let (dw, dh) = (x1 - x0, y1 - y0);
        if dw <= 0 || dh <= 0 {
            return;
        }
        if dw as usize == self.cell_w && dh as usize == self.cell_h {
            return self.draw_cell(fb, x0, y0, glyph, fg, bg);
        }
        fb.blend_rect(x0, y0, dw, dh, bg);
        if fg.a <= 0.0 {
            return;
        }
        let (gw, gh) = (self.glyph_w, self.glyph_h);
        let mask = &self.masks[(glyph as usize & 255) * gw * gh..][..gw * gh];
        let color = pack(fg.to_rgb());
        for py in y0.max(0)..y1.min(fb.height as i32) {
            let sy = ((py - y0) as usize * gh) / dh as usize;
            for px in x0.max(0)..x1.min(fb.width as i32) {
                let sx = ((px - x0) as usize * gw) / dw as usize;
                if mask[sy * gw + sx] {
                    let p = &mut fb.pixels[py as usize * fb.width + px as usize];
                    *p = blend(*p, color, fg.a);
                }
            }
        }
    }
}