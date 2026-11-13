use gpui_kit::component::{Theme, ThemeMode};
use gpui_kit::{App, Font, FontWeight, Hsla, Rgba, font as font_family, px};
use malmoon_tokens as tokens;

/// Converts a token color into GPUI's color type.
pub fn color(color: tokens::Color) -> Hsla {
    Rgba {
        r: f32::from(color.r) / 255.0,
        g: f32::from(color.g) / 255.0,
        b: f32::from(color.b) / 255.0,
        a: color.a,
    }
    .into()
}

/// The UI typeface at a token weight.
pub fn font(weight: f32) -> Font {
    Font {
        weight: FontWeight(weight),
        ..font_family(tokens::FONT_FAMILY_SANS[0])
    }
}

/// Sets up GPUI Component and applies Malmoon's tokens to its theme. Call once, before opening
/// any window.
pub fn init(cx: &mut App) {
    gpui_kit::init(cx);
    // The prompter is dark in both system appearances, and it is the only surface so far.
    Theme::change(ThemeMode::Dark, None, cx);

    let colors = tokens::colors(tokens::Mode::Dark);
    let theme = Theme::global_mut(cx);

    // GPUI Component paints this color under every window's root. Left opaque, it hides the
    // blurred backdrop of an overlay window, so each surface paints its own background instead.
    theme.tokens.background = gpui_kit::gpui::transparent_black().into();

    theme.font_family = tokens::FONT_FAMILY_SANS[0].into();
    theme.font_size = px(tokens::FONT_SIZE_MD);
    theme.radius = px(tokens::RADIUS_MD);
    theme.radius_lg = px(tokens::RADIUS_LG);

    theme.foreground = color(colors.text_primary);
    theme.muted_foreground = color(colors.text_secondary);
    theme.border = color(colors.border_default);
    theme.input = color(colors.border_default);
    theme.ring = color(colors.accent_default);
    theme.primary = color(colors.accent_default);
    theme.primary_foreground = color(colors.text_on_accent);
    theme.danger = color(colors.status_danger);
    theme.success = color(colors.status_success);
    theme.warning = color(colors.status_warning);
}
