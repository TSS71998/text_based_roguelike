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