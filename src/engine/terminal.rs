//! `Ctx`: the frame the game draws into each tick.
//!
//! The screen is a stack of layers, composited bottom to top into one framebuffer:
//! - `Simple`: a full grid of cells.
//! - `Sparse`: a grid where only cells that were set are drawn (for UI over other layers).
//! - `Fancy`: free-floating glyphs at fractional positions with a z-order (smooth movement).
//! - `Pixels`: a raw pixel buffer scaled into a screen rectangle (the first-person view).

use super::color::{RGB, RGBA};
use super::cp437::{FontCharType, to_cp437};
use super::font::Font;
use super::framebuffer::Framebuffer;
use super::geom::{Point, PointF};
use super::input::Key;
use super::rex::XpFile;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayerKind {
    Simple,
    Sparse,
    Fancy,
    Pixels {
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        width: usize,
        height: usize,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Cell {
    glyph: FontCharType,
    fg: RGBA,
    bg: RGBA,
}

const BLANK: Cell = Cell {
    glyph: 32,
    fg: RGBA::from_f32(1.0, 1.0, 1.0, 1.0),
    bg: RGBA::from_f32(0.0, 0.0, 0.0, 1.0),
};

#[derive(Clone, Copy, Debug)]
struct Sprite {
    pos: PointF,
    z: i32,
    scale: PointF,
    fg: RGBA,
    bg: RGBA,
    glyph: FontCharType,
}

enum Layer {
    Simple(Vec<Cell>),
    Sparse {
        cells: Vec<Option<Cell>>,
        used: Vec<usize>,
    },
    Fancy(Vec<Sprite>),
    Pixels {
        fb: Framebuffer,
        rect: (i32, i32, i32, i32),
        visible: bool,
    },
}

pub struct CtxBuilder {
    cols: i32,
    rows: i32,
    cell_w: usize,
    cell_h: usize,
    title: String,
    font: Option<(&'static [u8], usize, usize)>,
    layers: Vec<LayerKind>,
    scanlines: bool,
}

impl CtxBuilder {
    pub fn new(cols: i32, rows: i32) -> CtxBuilder {
        CtxBuilder {
            cols,
            rows,
            cell_w: 8,
            cell_h: 8,
            title: String::new(),
            font: None,
            layers: Vec::new(),
            scanlines: false,
        }
    }

    pub fn with_tile_dimensions(mut self, w: usize, h: usize) -> Self {
        self.cell_w = w;
        self.cell_h = h;
        self
    }

    pub fn with_title(mut self, title: &str) -> Self {
        self.title = title.to_string();
        self
    }

    pub fn with_font(mut self, png: &'static [u8], glyph_w: usize, glyph_h: usize) -> Self {
        self.font = Some((png, glyph_w, glyph_h));
        self
    }

    pub fn with_layer(mut self, kind: LayerKind) -> Self {
        self.layers.push(kind);
        self
    }

    pub fn with_scanlines(mut self, on: bool) -> Self {
        self.scanlines = on;
        self
    }

    pub fn build(self) -> Result<Ctx, String> {
        let (png, gw, gh) = self.font.ok_or("no font glyph")?;
        let mut font = Font::from_png(png, gw, gh)?;
        font.prepare_cell_size(self.cell_w, self.cell_h);
        let n = (self.cols * self.rows) as usize;
        let (cw, ch) = (self.cell_w as i32, self.cell_h as i32);
        let layers = self
            .layers
            .iter()
            .map(|k| match *k {
                LayerKind::Simple => Layer::Simple(vec![BLANK; n]),
                LayerKind::Sparse => Layer::Sparse {
                    cells: vec![None; n],
                    used: Vec::new(),
                },
                LayerKind::Fancy => Layer::Fancy(Vec::new()),
                LayerKind::Pixels {
                    x,
                    y,
                    w,
                    h,
                    width,
                    height,
                } => Layer::Pixels {
                    fb: Framebuffer::new(width, height),
                    rect: (x * cw, y * ch, w * cw, h * ch),
                    visible: false,
                },
            })
            .collect::<Vec<_>>();
        if layers.is_empty() {
            return Err("no layers given".into());
        }
        Ok(Ctx {
            title: self.title,
            cols: self.cols,
            rows: self.rows,
            cell_w: self.cell_w,
            cell_h: self.cell_h,
            screen: Framebuffer::new(
                self.cols as usize * self.cell_w,
                self.rows as usize * self.cell_h,
            ),
            font,
            layers,
            active: 0,
            scanlines: self.scanlines,
            burn: Vec::new(),
            key: None,
            shift: false,
            control: false,
            alt: false,
            mouse_px: (0, 0),
            left_click: false,
            frame_time_ms: 0.0,
            quitting: false,
        })
    }
}

pub struct Ctx {
    pub title: String,
    cols: i32,
    rows: i32,
    cell_w: usize,
    cell_h: usize,
    screen: Framebuffer,
    font: Font,
    layers: Vec<Layer>,
    active: usize,
    pub scanlines: bool,
    burn: Vec<u32>,

    pub key: Option<Key>,
    pub shift: bool,
    pub control: bool,
    pub alt: bool,
    pub mouse_px: (i32, i32),
    pub left_click: bool,
    pub frame_time_ms: f32,
    pub quitting: bool,
}

impl Ctx {
    pub fn set_active_console(&mut self, layer: usize) {
        assert!(layer < self.layers.len(), "no layer {layer}");
        self.active = layer;
    }

    pub fn cls(&mut self) {
        match &mut self.layers[self.active] {
            Layer::Simple(cells) => cells.fill(BLANK),
            Layer::Sparse { cells, used } => {
                for &i in used.iter() {
                    cells[i] = None;
                }
                used.clear();
            }
            Layer::Fancy(sprites) => sprites.clear(),
            Layer::Pixels { fb, .. } => fb.clear(0),
        }
    }

    pub fn get_char_size(&self) -> (u32, u32) {
        (self.cols as u32, self.rows as u32)
    }

    pub fn cell_size(&self) -> (usize, usize) {
        (self.cell_w, self.cell_h)
    }

    pub fn pixels_mut(&mut self, layer: usize) -> Option<&mut Framebuffer> {
        match self.layers.get_mut(layer) {
            Some(Layer::Pixels { fb, .. }) => Some(fb),
            _ => None,
        }
    }

    pub fn set_layer_visible(&mut self, layer: usize, on: bool) {
        if let Some(Layer::Pixels { visible, .. }) = self.layers.get_mut(layer) {
            *visible = on;
        }
    }

    fn put_cell(&mut self, x: i32, y: i32, cell: Cell) {
        if x < 0 || y < 0 || x >= self.cols || y >= self.rows {
            return;
        }
        let i = (y * self.cols + x) as usize;
        match &mut self.layers[self.active] {
            Layer::Simple(cells) => cells[i] = cell,
            Layer::Sparse { cells, used } => {
                if cells[i].is_none() {
                    used.push(i);
                }
                cells[i] = Some(cell);
            }
            Layer::Fancy(sprites) => sprites.push(Sprite {
                pos: PointF::new(x as f32, y as f32),
                z: 0,
                scale: PointF::new(1.0, 1.0),
                fg: cell.fg,
                bg: cell.bg,
                glyph: cell.glyph,
            }),
            Layer::Pixels { .. } => {}
        }
    }

    pub fn set(
        &mut self,
        x: i32,
        y: i32,
        fg: impl Into<RGBA>,
        bg: impl Into<RGBA>,
        glyph: FontCharType,
    ) {
        self.put_cell(
            x,
            y,
            Cell {
                glyph,
                fg: fg.into(),
                bg: bg.into(),
            },
        );
    }

    pub fn set_bg(&mut self, x: i32, y: i32, bg: impl Into<RGBA>) {
        self.put_cell(
            x,
            y,
            Cell {
                glyph: 32,
                fg: RGBA::from_f32(0.0, 0.0, 0.0, 0.0),
                bg: bg.into(),
            },
        );
    }

    pub fn set_fancy(
        &mut self,
        pos: PointF,
        z: i32,
        scale: PointF,
        fg: impl Into<RGBA>,
        bg: impl Into<RGBA>,
        glyph: FontCharType,
    ) {
        if let Layer::Fancy(sprites) = &mut self.layers[self.active] {
            sprites.push(Sprite {
                pos,
                z,
                scale,
                fg: fg.into(),
                bg: bg.into(),
                glyph,
            });
        }
    }

    pub fn print(&mut self, x: i32, y: i32, text: impl AsRef<str>) {
        self.print_color(
            x,
            y,
            RGB::from_f32(1.0, 1.0, 1.0),
            RGB::from_f32(0.0, 0.0, 0.0),
            text,
        );
    }

    pub fn print_color(
        &mut self,
        x: i32,
        y: i32,
        fg: impl Into<RGBA>,
        bg: impl Into<RGBA>,
        text: impl AsRef<str>,
    ) {
        let (fg, bg) = (fg.into(), bg.into());
        for (i, c) in text.as_ref().chars().enumerate() {
            self.put_cell(
                x + i as i32,
                y,
                Cell {
                    glyph: to_cp437(c),
                    fg,
                    bg,
                },
            );
        }
    }

    pub fn print_color_centered(
        &mut self,
        y: i32,
        fg: impl Into<RGBA>,
        bg: impl Into<RGBA>,
        text: impl AsRef<str>,
    ) {
        let len = text.as_ref().chars().count() as i32;
        self.print_color(self.cols / 2 - len / 2, y, fg, bg, text);
    }

    pub fn draw_box(
        &mut self,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        fg: impl Into<RGBA>,
        bg: impl Into<RGBA>,
    ) {
        self.draw_box_with(
            x,
            y,
            w,
            h,
            fg.into(),
            bg.into(),
            ['┌', '┐', '└', '┘', '─', '│'],
        );
    }

    pub fn draw_box_double(
        &mut self,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        fg: impl Into<RGBA>,
        bg: impl Into<RGBA>,
    ) {
        self.draw_box_with(
            x,
            y,
            w,
            h,
            fg.into(),
            bg.into(),
            ['╔', '╗', '╚', '╝', '═', '║'],
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_box_with(&mut self, x: i32, y: i32, w: i32, h: i32, fg: RGBA, bg: RGBA, c: [char; 6]) {
        for cy in y..=y + h {
            for cx in x..=x + w {
                self.put_cell(cx, cy, Cell { glyph: 32, fg, bg });
            }
        }
        let mut put = |cx, cy, ch| {
            self.put_cell(
                cx,
                cy,
                Cell {
                    glyph: to_cp437(ch),
                    fg,
                    bg,
                },
            )
        };
        for cx in x + 1..x + w {
            put(cx, y, c[4]);
            put(cx, y + h, c[4]);
        }
        for cy in y + 1..y + h {
            put(x, cy, c[5]);
            put(x + w, cy, c[5]);
        }
        put(x, y, c[0]);
        put(x + w, y, c[1]);
        put(x, y + h, c[2]);
        put(x + w, y + h, c[3]);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw_bar_horizontal(
        &mut self,
        x: i32,
        y: i32,
        width: i32,
        n: i32,
        max: i32,
        fg: impl Into<RGBA>,
        bg: impl Into<RGBA>,
    ) {
        let (fg, bg) = (fg.into(), bg.into());
        let filled = if max > 0 {
            (n as f32 / max as f32 * width as f32) as i32
        } else {
            0
        };
        for i in 0..width {
            let glyph = to_cp437(if i <= filled { '▓' } else { '░' });
            self.put_cell(x + i, y, Cell { glyph, fg, bg });
        }
    }

    pub fn render_xp_sprite(&mut self, xp: &XpFile, x: i32, y: i32) {
        for layer in &xp.layers {
            for cy in 0..layer.height {
                for cx in 0..layer.width {
                    let cell = layer.get(cx, cy).unwrap();
                    if !cell.bg.is_transparent() {
                        self.set(
                            x + cx as i32,
                            y + cy as i32,
                            cell.fg.to_rgb(),
                            cell.bg.to_rgb(),
                            cell.ch as FontCharType,
                        );
                    }
                }
            }
        }
    }

    pub fn mouse_pos(&self) -> (i32, i32) {
        (
            self.mouse_px.0 / self.cell_w as i32,
            self.mouse_px.1 / self.cell_h as i32,
        )
    }

    pub fn mouse_point(&self) -> Point {
        let (x, y) = self.mouse_pos();
        Point::new(x, y)
    }

    pub fn screen(&self) -> &Framebuffer {
        &self.screen
    }

    pub fn render(&mut self) -> &Framebuffer {
        let (cw, ch) = (self.cell_w as i32, self.cell_h as i32);
        let cols = self.cols as usize;
        self.screen.clear(0);
        let mut pixel_rect = None;
        for layer in self.layers.iter_mut() {
            match layer {
                Layer::Simple(cells) => {
                    for (i, c) in cells.iter().enumerate() {
                        let blank = (c.glyph == 32 || c.glyph == 0) && c.bg == BLANK.bg;
                        if !blank {
                            let (x, y) = ((i % cols) as i32, (i / cols) as i32);
                            self.font.draw_cell(
                                &mut self.screen,
                                x * cw,
                                y * ch,
                                c.glyph,
                                c.fg,
                                c.bg,
                            );
                        }
                    }
                }
                Layer::Sparse { cells, used } => {
                    for &i in used.iter() {
                        if let Some(c) = cells[i] {
                            let (x, y) = ((i % cols) as i32, (i / cols) as i32);
                            self.font.draw_cell(
                                &mut self.screen,
                                x * cw,
                                y * ch,
                                c.glyph,
                                c.fg,
                                c.bg,
                            );
                        }
                    }
                }
                Layer::Fancy(sprites) => {
                    sprites.sort_by_key(|s| s.z);
                    for s in sprites.iter() {
                        let (w, h) = (cw as f32 * s.scale.x, ch as f32 * s.scale.y);
                        let x = s.pos.x * cw as f32 + (cw as f32 - w) / 2.0;
                        let y = s.pos.y * ch as f32 + (ch as f32 - h) / 2.0;
                        self.font
                            .draw_scaled(&mut self.screen, x, y, w, h, s.glyph, s.fg, s.bg);
                    }
                }
                Layer::Pixels { fb, rect, visible } => {
                    if *visible {
                        self.screen.blit_scaled(fb, rect.0, rect.1, rect.2, rect.3);
                        pixel_rect = Some(*rect);
                    }
                }
            }
        }
        if self.scanlines {
            if self.burn.len() != self.screen.pixels.len() {
                self.burn = Framebuffer::screen_burn(self.screen.width, self.screen.height);
            }
            self.screen.crt(&self.burn, pixel_rect);
        }

        &self.screen
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::color::{BLACK, WHITE, YELLOW};

    fn ctx() -> Ctx {
        CtxBuilder::new(10, 4)
            .with_tile_dimensions(16, 16)
            .with_font(include_bytes!("../../resources/terminal8x8.png"), 8, 8)
            .with_layer(LayerKind::Simple)
            .with_layer(LayerKind::Pixels {
                x: 0,
                y: 0,
                w: 2,
                h: 1,
                width: 4,
                height: 2,
            })
            .with_layer(LayerKind::Fancy)
            .with_layer(LayerKind::Sparse)
            .build()
            .unwrap()
    }

    #[test]
    fn layers_composite_in_order() {
        let mut c = ctx();
        c.set_active_console(0);
        c.set(0, 0, RGB::named(WHITE), RGB::named(BLACK), to_cp437('#'));
        c.set_active_console(3);
        c.print_color(1, 0, RGB::named(YELLOW), RGB::named(BLACK), "@");
        c.set_active_console(2);
        c.set_fancy(
            PointF::new(3.5, 0.0),
            1,
            PointF::new(1.0, 1.0),
            RGB::named(WHITE),
            RGBA::from_f32(0.0, 0.0, 0.0, 0.0),
            to_cp437('o'),
        );
        let lit = |c: &mut Ctx, x0: i32, x1: i32, color: u32| {
            let s = c.render();
            (x0..x1)
                .flat_map(|x| (0..16).map(move |y| (x, y)))
                .filter(|&(x, y)| s.get(x, y) == color)
                .count()
        };
        assert!(lit(&mut c, 0, 16, 0xFFFFFF) > 10, "'#' drawn in white");
        assert!(lit(&mut c, 16, 32, 0xFFFF00) > 10, "'@' drawn in yellow");
        assert!(
            lit(&mut c, 56, 72, 0xFFFFFF) > 5,
            "fancy 'o' drawn at x = 3.5 cells"
        );

        // A visible pixel layer covers what's below it.
        c.pixels_mut(1).unwrap().clear(0x0000FF);
        c.set_layer_visible(1, true);
        assert!(lit(&mut c, 0, 16, 0x0000FF) > 200);
        // ...but the sparse UI layer still draws on top.
        assert!(lit(&mut c, 16, 32, 0xFFFF00) > 10);
    }

    #[test]
    fn box_matches_rltk_extents() {
        let mut c = ctx();
        c.set_active_console(3);
        c.draw_box(0, 0, 3, 2, RGB::named(WHITE), RGB::named(BLACK));
        let s = c.render();
        // Right edge is at x = 3 (inclusive): the vertical bar's pixels are in cell 3.
        assert!((48..64).any(|x| s.get(x, 24) == 0xFFFFFF));
        assert!((64..80).all(|x| s.get(x, 24) == 0));
    }

    #[test]
    fn sparse_cls_only_clears_used_cells() {
        let mut c = ctx();
        c.set_active_console(3);
        c.print(0, 0, "hi");
        c.cls();
        assert!(c.render().pixels.iter().all(|&p| p == 0));
    }
}

#[cfg(test)]
mod bench {
    use super::*;

    /// `cargo test --release frame_cost -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn frame_cost() {
        let mut c = CtxBuilder::new(80, 60)
            .with_tile_dimensions(16, 16)
            .with_font(include_bytes!("../../resources/terminal8x8.png"), 8, 8)
            .with_layer(LayerKind::Simple)
            .with_layer(LayerKind::Pixels {
                x: 1,
                y: 1,
                w: 48,
                h: 44,
                width: 384,
                height: 352,
            })
            .with_layer(LayerKind::Fancy)
            .with_layer(LayerKind::Sparse)
            .with_scanlines(true)
            .build()
            .unwrap();
        let frame = |c: &mut Ctx, pixels: bool| {
            c.set_active_console(2);
            c.cls();
            for y in 0..45 {
                for x in 0..49 {
                    c.set_fancy(
                        PointF::new(x as f32 - 0.3, y as f32 + 0.6),
                        0,
                        PointF::new(1.0, 1.0),
                        RGB::from_f32(0.7, 0.7, 0.7),
                        RGB::from_f32(0.1, 0.0, 0.0),
                        to_cp437('#'),
                    );
                }
            }
            c.set_active_console(3);
            c.cls();
            c.draw_box(
                0,
                0,
                79,
                59,
                RGB::from_f32(0.6, 0.6, 0.6),
                RGB::from_f32(0.0, 0.0, 0.0),
            );
            for y in 46..58 {
                c.print(2, y, "Patron says \"Quiet down, it's too early!\"");
            }
            c.set_layer_visible(1, pixels);
            c.render();
        };
        c.set_active_console(2);
        c.cls();
        c.set_active_console(3);
        c.cls();
        let start = std::time::Instant::now();
        for _ in 0..100 {
            c.render();
        }
        println!(
            "empty frame (clear + CRT): {:.2} ms",
            start.elapsed().as_secs_f64() * 10.0
        );
        c.scanlines = false;
        let start = std::time::Instant::now();
        for _ in 0..100 {
            frame(&mut c, false);
        }
        println!(
            "busy frame without CRT: {:.2} ms",
            start.elapsed().as_secs_f64() * 10.0
        );
        c.scanlines = true;
        for pixels in [false, true] {
            let start = std::time::Instant::now();
            for _ in 0..100 {
                frame(&mut c, pixels);
            }
            println!(
                "frame (3D layer {}): {:.2} ms",
                if pixels { "on" } else { "off" },
                start.elapsed().as_secs_f64() * 10.0
            );
        }
    }
}
