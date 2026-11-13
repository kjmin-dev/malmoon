//! Malmoon design tokens as Rust constants.
//!
//! The values live in `design/tokens/*.json`. `mise run //:tokens` regenerates
//! [`generated`](self) from them, and CI fails if the committed file is out of date. This crate
//! has no dependencies, so any UI layer can convert the plain types below into its own.

#![forbid(unsafe_code)]

// Generated code is laid out by its generator, not by rustfmt.
#[rustfmt::skip]
mod generated;

pub use generated::*;

/// An sRGB color with straight (non-premultiplied) alpha.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    /// 0.0 is fully transparent, 1.0 fully opaque.
    pub a: f32,
}

impl Color {
    pub const fn rgba(r: u8, g: u8, b: u8, a: f32) -> Self {
        Self { r, g, b, a }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mode {
    Light,
    Dark,
}

/// The color tokens for one [`Mode`].
pub const fn colors(mode: Mode) -> &'static Colors {
    match mode {
        Mode::Light => &LIGHT,
        Mode::Dark => &DARK,
    }
}

/// A cubic Bézier easing curve: `[x1, y1, x2, y2]`.
pub type CubicBezier = [f32; 4];

/// One layer of a drop shadow. Lengths are in logical pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shadow {
    pub color: Color,
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur: f32,
    pub spread: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_modes_are_generated() {
        assert_ne!(
            colors(Mode::Light).background_base,
            colors(Mode::Dark).background_base
        );
    }
}
