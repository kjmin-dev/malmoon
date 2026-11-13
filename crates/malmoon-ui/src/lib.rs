//! Malmoon's GPUI components and the bridge from design tokens to GPUI.
//!
//! GPUI Component supplies behavior (focus, text input, IME); this crate supplies Malmoon's look
//! by mapping `malmoon-tokens` onto it, plus the product components built on top.

#![forbid(unsafe_code)]

mod caption_stream;
mod components;
mod theme;

pub use caption_stream::CaptionStream;
pub use components::{
    Badge, BadgeTone, Banner, BannerTone, ButtonVariant, ControlSize, TextField, button, card,
    divider, text,
};
pub use theme::{TypographyExt, color, font, init, mode, palette, set_mode};
