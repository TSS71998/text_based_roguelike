use serde::{Deserialize, Serialize};
use std::ops::{Add, AddAssign, Mul, MulAssign, Sub};

pub type ColorU8 = (u8, u8, u8);

pub const BLACK: ColorU8 = (0, 0, 0);
pub const WHITE: ColorU8 = (255, 255, 255);
pub const GRAY: ColorU8 = (128, 128, 128);
pub const RED: ColorU8 = (255, 0, 0);
pub const GREEN: ColorU8 = (0, 255, 0);
pub const BLUE: ColorU8 = (0, 0, 255);
pub const NAVY_BLUE: ColorU8 = (0, 0, 128);
pub const CYAN: ColorU8 = (0, 255, 255);
pub const MAGENTA: ColorU8 = (255, 0, 255);
pub const YELLOW: ColorU8 = (255, 255, 0);
pub const ORANGE: ColorU8 = (255, 165, 0);
pub const GOLD: ColorU8 = (255, 215, 0);
pub const CHOCOLATE: ColorU8 = (210, 105, 30);
pub const WHEAT: ColorU8 = (245, 222, 179);

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RGB {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RGBA {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

#[derive(Debug)]
pub struct HexParseError;

impl std::fmt::Display for HexParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("expected a color like #rrggbb")
    }
}

impl std::error::Error for HexParseError {}

impl RGB {
    pub const fn from_f32(r: f32, g: f32, b: f32) -> RGB {
        RGB { r, g, b}
    }

    pub fn from_u8(r: u8, g: u8, b: u8) -> RGB {
        RGB { r: r as f32 / 255.0, g: g as f32 / 255.0, b: b as f32 / 255.0 }
    }

    pub fn named(c: ColorU8) -> RGB {
        RGB::from_u8(c.0, c.1, c.2)
    }

    pub fn from_hex(code: &str) -> Result<RGB, HexParseError> {
        let hex = code.strip_prefix('#').unwrap_or(code);
        if hex.len() != 6 || !hex.is_ascii() {
            return Err(HexParseError);
        }
        let channel = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|_| HexParseError);
        Ok(RGB::from_u8(channel(0)?, channel(2)?, channel(4)?))
    }

    pub fn lerp(&self, other: RGB, t: f32) -> RGB {
        RGB {
            r: self.r + (other.r - self.r) * t,
            g: self.g + (other.g - self.g) * t,
            b: self.b + (other.b - self.b) * t,
        }
    }

    pub fn to_greyscale(&self) -> RGB {
        let l = self.r * 0.299 + self.g * 0.587 + self.b * 0.114;
        RGB { r: l, g: l, b: l }
    }

    pub fn to_rgba(&self, a: f32) -> RGBA {
        RGBA { r: self.r, g: self.g, b: self.b, a }
    }
}

impl RGBA {
    pub const fn from_f32(r: f32, g: f32, b: f32, a: f32) -> RGBA {
        RGBA { r, g, b, a }
    }

    pub fn named(c: ColorU8) -> RGBA {
        RGB::named(c).to_rgba(1.0)
    }

    pub fn to_rgb(&self) -> RGB {
        RGB { r: self.r, g: self.r, b: self.b }
    }
}

impl From<RGB> for RGBA {
    fn from(c: RGB) -> Self {
        c.to_rgba(1.0)
    }
}

impl From<ColorU8> for RGBA {
    fn from(c: ColorU8) -> Self {
        RGBA::named(c)
    }
}

impl From<ColorU8> for RGB {
    fn from(c: ColorU8) -> Self {
        RGB::named(c)
    }
}

impl Add for RGB {
    type Output = RGB;
    fn add(self, o: RGB) -> RGB {
        RGB { r: self.r + o.r, g: self.g + o.g, b: self.b + o.b }
    }
}

impl AddAssign for RGB {
    fn add_assign(&mut self, o: RGB) {
        *self = *self + o;
    }
}

impl Sub for RGB {
    type Output = RGB;
    fn sub(self, o: RGB) -> RGB {
        RGB { r: self.r - o.r, g: self.g - o.g, b: self.b - o.b }
    }
}

impl Mul for RGB {
    type Output = RGB;
    fn mul(self, o: RGB) -> RGB {
        RGB { r: self.r * o.r, g: self.g * o.g, b: self.b * o.b }
    }
}

impl Mul<f32> for RGB {
    type Output = RGB;
    fn mul(self, s: f32) -> RGB {
        RGB { r: self.r * s, g: self.g * s, b: self.b * s }
    }
}

impl MulAssign<f32> for RGB {
    fn mul_assign(&mut self, s: f32) {
        *self = *self * s;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
 
    #[test]
    fn hex_parses() {
        assert_eq!(RGB::from_hex("#ff8000").unwrap(), RGB::from_u8(255, 128, 0));
        assert_eq!(RGB::from_hex("00FF00").unwrap(), RGB::named(GREEN));
        assert!(RGB::from_hex("#fff").is_err());
        assert!(RGB::from_hex("#gg0000").is_err());
    }
}