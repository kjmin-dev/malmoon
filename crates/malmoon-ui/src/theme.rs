use gpui_kit::component::{Theme, ThemeMode};
use gpui_kit::{App, Font, FontWeight, Hsla, Rgba, Styled, font as font_family, px};
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

/// Applies a text role: size, weight and line height together.
pub trait TypographyExt: Styled + Sized {
    fn typography(self, style: tokens::Typography) -> Self {
        self.text_size(px(style.size))
            .font_weight(FontWeight(style.weight))
            .line_height(px(style.line_height_px()))
    }
}

impl<T: Styled> TypographyExt for T {}

/// Sets up GPUI Component and applies Malmoon's tokens to its theme. Call once, before opening
/// any window. The app starts dark: the prompter is dark in both system appearances.
pub fn init(cx: &mut App) {
    gpui_kit::init(cx);
    set_mode(tokens::Mode::Dark, cx);
}

/// The color mode the components draw in.
pub fn mode(cx: &App) -> tokens::Mode {
    if Theme::global(cx).is_dark() {
        tokens::Mode::Dark
    } else {
        tokens::Mode::Light
    }
}

/// The color tokens for the current mode.
pub fn palette(cx: &App) -> &'static tokens::Colors {
    tokens::colors(mode(cx))
}

/// Switches every component to light or dark.
pub fn set_mode(mode: tokens::Mode, cx: &mut App) {
    let theme_mode = match mode {
        tokens::Mode::Light => ThemeMode::Light,
        tokens::Mode::Dark => ThemeMode::Dark,
    };
    Theme::change(theme_mode, None, cx);

    let c = tokens::colors(mode);
    let theme = Theme::global_mut(cx);

    // GPUI Component paints this color under every window's root. Left opaque, it hides the
    // blurred backdrop of an overlay window, so each surface paints its own background instead.
    theme.tokens.background = gpui_kit::gpui::transparent_black().into();

    theme.font_family = tokens::FONT_FAMILY_SANS[0].into();
    theme.font_size = px(tokens::TYPOGRAPHY_BODY.size);
    theme.radius = px(tokens::RADIUS_MD);
    theme.radius_lg = px(tokens::RADIUS_LG);

    // GPUI Component keeps every color twice: as a plain color, and in `tokens` as a paint,
    // which is what most components draw with. Both are set, so nothing reads a stale default.
    macro_rules! paint {
        ($($field:ident = $token:expr;)+) => {$(
            let value = color($token);
            theme.$field = value;
            theme.tokens.$field = value.into();
        )+};
    }
    paint! {
        // Text and lines.
        foreground = c.text_primary;
        muted = c.control_default;
        muted_foreground = c.text_secondary;
        border = c.border_default;
        input = c.field_border;
        ring = c.border_focus;
        caret = c.text_primary;
        link = c.text_accent;
        link_hover = c.text_accent;
        link_active = c.text_accent;
        popover = c.background_raised;
        popover_foreground = c.text_primary;

        // Buttons, under both of the names GPUI Component has for each role.
        primary = c.button_primary_background;
        primary_hover = c.button_primary_background_hover;
        primary_active = c.button_primary_background_pressed;
        primary_foreground = c.button_primary_text;
        button_primary = c.button_primary_background;
        button_primary_hover = c.button_primary_background_hover;
        button_primary_active = c.button_primary_background_pressed;
        button_primary_foreground = c.button_primary_text;

        secondary = c.button_secondary_background;
        secondary_hover = c.button_secondary_background_hover;
        secondary_active = c.button_secondary_background_pressed;
        secondary_foreground = c.button_secondary_text;
        button_secondary = c.button_secondary_background;
        button_secondary_hover = c.button_secondary_background_hover;
        button_secondary_active = c.button_secondary_background_pressed;
        button_secondary_foreground = c.button_secondary_text;

        danger = c.button_danger_background;
        danger_hover = c.button_danger_background_hover;
        danger_active = c.button_danger_background_pressed;
        danger_foreground = c.button_danger_text;
        button_danger = c.button_danger_background;
        button_danger_hover = c.button_danger_background_hover;
        button_danger_active = c.button_danger_background_pressed;
        button_danger_foreground = c.button_danger_text;

        // What a ghost button and a hovered list row fill with.
        accent = c.button_ghost_background_hover;
        accent_foreground = c.button_ghost_text;

        success = c.status_success;
        success_foreground = c.background_base;
        warning = c.status_warning;
        warning_foreground = c.background_base;
        info = c.status_info;
        info_foreground = c.background_base;
    }
}
