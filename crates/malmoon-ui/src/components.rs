//! Malmoon's general-purpose components for GPUI.
//!
//! They take the same variants, sizes and tones as the web components in `@malmoon/ui` and the
//! React Native ones in `@malmoon/ui-native`, and draw them from the same design tokens.
//! Interactive parts come from GPUI Component, which supplies focus, keyboard and input-method
//! handling; this module gives them Malmoon's sizes and wraps what GPUI Component lacks.

use gpui_kit::component::{
    button::{Button, ButtonVariants},
    input::{Input, InputState},
};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    App, Div, ElementId, Entity, IntoElement, ParentElement, Pixels, RenderOnce, SharedString,
    Styled, Window, div, px,
};
use malmoon_tokens as tokens;

use crate::theme::{TypographyExt, color, palette};

// ---------------------------------------------------------------- button

/// `Primary` is the one action a view is for; use it once per view. `Secondary` is any other
/// action, `Ghost` an action that should stay out of the way, `Danger` one that destroys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonVariant {
    Primary,
    #[default]
    Secondary,
    Ghost,
    Danger,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ControlSize {
    Sm,
    #[default]
    Md,
    Lg,
}

impl ControlSize {
    /// The height of a control of this size. The desktop is used with a pointer.
    pub fn height(self) -> Pixels {
        px(match self {
            ControlSize::Sm => tokens::POINTER.control_sm,
            ControlSize::Md => tokens::POINTER.control_md,
            ControlSize::Lg => tokens::POINTER.control_lg,
        })
    }

    fn padding(self) -> Pixels {
        px(match self {
            ControlSize::Sm => tokens::SPACE_3,
            ControlSize::Md => tokens::SPACE_4,
            ControlSize::Lg => tokens::SPACE_6,
        })
    }
}

/// A GPUI Component button in one of Malmoon's variants and sizes. The result is a normal
/// `Button`: add `.on_click`, `.disabled` or `.loading` to it.
pub fn button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    variant: ButtonVariant,
    size: ControlSize,
) -> Button {
    let button = Button::new(id).label(label);
    let button = match variant {
        ButtonVariant::Primary => button.primary(),
        ButtonVariant::Secondary => button.secondary(),
        ButtonVariant::Ghost => button.ghost(),
        ButtonVariant::Danger => button.danger(),
    };
    let label = match size {
        ControlSize::Lg => tokens::TYPOGRAPHY_BODY_STRONG,
        _ => tokens::TYPOGRAPHY_LABEL,
    };
    button
        .h(size.height())
        .min_w(size.height())
        .px(size.padding())
        .typography(label)
}

// ---------------------------------------------------------------- text field

/// A labelled single-line text field. The label is always visible; a placeholder is not a
/// label. `error` replaces `description` while there is one.
#[derive(IntoElement)]
pub struct TextField {
    label: SharedString,
    state: Entity<InputState>,
    description: Option<SharedString>,
    error: Option<SharedString>,
    disabled: bool,
}

impl TextField {
    pub fn new(label: impl Into<SharedString>, state: &Entity<InputState>) -> Self {
        Self {
            label: label.into(),
            state: state.clone(),
            description: None,
            error: None,
            disabled: false,
        }
    }

    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn error(mut self, error: Option<impl Into<SharedString>>) -> Self {
        self.error = error.map(Into::into);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl RenderOnce for TextField {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = palette(cx);
        let invalid = self.error.is_some();
        let note = self.error.or(self.description).map(|text| {
            let tone = if invalid {
                colors.status_danger
            } else {
                colors.text_secondary
            };
            div()
                .typography(tokens::TYPOGRAPHY_DETAIL)
                .text_color(color(tone))
                .child(text)
        });
        div()
            .flex()
            .flex_col()
            .gap(px(tokens::SPACE_1))
            .child(
                div()
                    .typography(tokens::TYPOGRAPHY_LABEL)
                    .text_color(color(if self.disabled {
                        colors.text_secondary
                    } else {
                        colors.text_primary
                    }))
                    .child(self.label.clone()),
            )
            .child(
                // An outline of its own when invalid, thicker than the field's, so the state
                // does not depend on telling red from grey.
                div()
                    .rounded(px(tokens::RADIUS_MD))
                    .when(invalid, |outline| {
                        outline
                            .border(px(tokens::BORDER_WIDTH_THIN))
                            .border_color(color(colors.field_border_invalid))
                    })
                    .child(
                        Input::new(&self.state)
                            .aria_label(self.label)
                            .h(ControlSize::Md.height())
                            .disabled(self.disabled),
                    ),
            )
            .children(note)
    }
}

// ---------------------------------------------------------------- badge

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BadgeTone {
    #[default]
    Neutral,
    Accent,
    Success,
    Warning,
    Danger,
}

/// A short status or category. The words carry the meaning; the color only reinforces it.
/// `dot` adds a small mark for live states such as "connected".
#[derive(IntoElement)]
pub struct Badge {
    label: SharedString,
    tone: BadgeTone,
    dot: bool,
}

impl Badge {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            tone: BadgeTone::default(),
            dot: false,
        }
    }

    pub fn tone(mut self, tone: BadgeTone) -> Self {
        self.tone = tone;
        self
    }

    pub fn dot(mut self) -> Self {
        self.dot = true;
        self
    }
}

impl RenderOnce for Badge {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let c = palette(cx);
        let (background, text) = match self.tone {
            BadgeTone::Neutral => (c.badge_neutral_background, c.badge_neutral_text),
            BadgeTone::Accent => (c.badge_accent_background, c.badge_accent_text),
            BadgeTone::Success => (c.badge_success_background, c.badge_success_text),
            BadgeTone::Warning => (c.badge_warning_background, c.badge_warning_text),
            BadgeTone::Danger => (c.badge_danger_background, c.badge_danger_text),
        };
        div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(6.0))
            .h(px(24.0))
            .px(px(tokens::SPACE_2))
            .rounded(px(tokens::RADIUS_SM))
            .bg(color(background))
            .text_color(color(text))
            .typography(tokens::TYPOGRAPHY_FOOTNOTE)
            .when(self.dot, |badge| {
                badge.child(
                    div()
                        .flex_none()
                        .size(px(6.0))
                        .rounded_full()
                        .bg(color(text)),
                )
            })
            .child(self.label)
    }
}

// ---------------------------------------------------------------- banner

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BannerTone {
    #[default]
    Info,
    Success,
    Warning,
    Danger,
}

/// A message about the whole view or a section of it. Each tone has its own mark, so the tone
/// does not depend on color.
#[derive(IntoElement)]
pub struct Banner {
    tone: BannerTone,
    title: Option<SharedString>,
    message: SharedString,
}

impl Banner {
    pub fn new(tone: BannerTone, message: impl Into<SharedString>) -> Self {
        Self {
            tone,
            title: None,
            message: message.into(),
        }
    }

    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }
}

impl RenderOnce for Banner {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let c = palette(cx);
        let (background, accent, mark) = match self.tone {
            BannerTone::Info => (c.banner_info_background, c.banner_info_accent, "i"),
            BannerTone::Success => (c.banner_success_background, c.banner_success_accent, "✓"),
            BannerTone::Warning => (c.banner_warning_background, c.banner_warning_accent, "!"),
            BannerTone::Danger => (c.banner_danger_background, c.banner_danger_accent, "×"),
        };
        div()
            .flex()
            .items_start()
            .gap(px(tokens::SPACE_3))
            .p(px(tokens::SPACE_3))
            .rounded(px(tokens::RADIUS_MD))
            .bg(color(background))
            .text_color(color(c.banner_text))
            .child(
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .justify_center()
                    .size(px(20.0))
                    .rounded_full()
                    .bg(color(accent))
                    .text_color(color(background))
                    .typography(tokens::TYPOGRAPHY_FOOTNOTE)
                    .child(mark),
            )
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .children(
                        self.title
                            .map(|title| div().typography(tokens::TYPOGRAPHY_LABEL).child(title)),
                    )
                    .child(
                        div()
                            .typography(tokens::TYPOGRAPHY_DETAIL)
                            .child(self.message),
                    ),
            )
    }
}

// ---------------------------------------------------------------- surfaces and text

/// A surface that groups related content. The outline, not a shadow, sets it apart: shadows
/// barely show in dark mode. Add the content as children.
pub fn card(cx: &App) -> Div {
    let c = palette(cx);
    div()
        .flex()
        .flex_col()
        .gap(px(tokens::SPACE_4))
        .p(px(tokens::SPACE_4))
        .rounded(px(tokens::RADIUS_LG))
        .border(px(tokens::BORDER_WIDTH_THIN))
        .border_color(color(c.border_default))
        .bg(color(c.background_surface))
}

/// A line between groups of content.
pub fn divider(cx: &App) -> Div {
    div()
        .flex_none()
        .h(px(tokens::BORDER_WIDTH_THIN))
        .w_full()
        .bg(color(palette(cx).border_default))
}

/// Text in one of the design system's roles, for example `tokens::TYPOGRAPHY_HEADING`.
pub fn text(content: impl Into<SharedString>, style: tokens::Typography) -> Div {
    div().typography(style).child(content.into())
}
