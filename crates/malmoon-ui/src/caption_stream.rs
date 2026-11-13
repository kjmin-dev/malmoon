use gpui_kit::{
    App, Font, IntoElement, ParentElement, Pixels, RenderOnce, SharedString, Styled, TextRun,
    Window, div, px,
};
use malmoon_core::{
    linebreak::wrap_words,
    protocol::{CaptionStage, CaptionUpdate},
};
use malmoon_tokens as tokens;

use crate::theme::{color, font};

/// Live captions: what was said, and its translation once there is one.
///
/// Draft segments are dimmed because recognition may still revise them. Lines are broken here,
/// between words, rather than left to GPUI, which breaks Korean in the middle of a word.
#[derive(IntoElement)]
pub struct CaptionStream {
    segments: Vec<CaptionUpdate>,
    width: Pixels,
}

impl CaptionStream {
    /// `width` is the width available to a line of text.
    pub fn new(segments: &[CaptionUpdate], width: Pixels) -> Self {
        Self {
            segments: segments.to_vec(),
            width,
        }
    }
}

fn wrap(
    text: &str,
    font: &Font,
    size: Pixels,
    width: Pixels,
    window: &Window,
) -> Vec<SharedString> {
    let measure = |candidate: &str| {
        let run = TextRun {
            len: candidate.len(),
            font: font.clone(),
            color: Default::default(),
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let line = window
            .text_system()
            .shape_line(candidate.to_owned().into(), size, &[run], None);
        line.width.as_f32()
    };
    wrap_words(text, width.as_f32(), measure)
        .into_iter()
        .map(SharedString::from)
        .collect()
}

impl RenderOnce for CaptionStream {
    fn render(self, window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let colors = tokens::colors(tokens::Mode::Dark);
        let caption_font = font(tokens::TYPOGRAPHY_CAPTION.weight);
        let caption_size = px(tokens::TYPOGRAPHY_CAPTION.size);
        let translation_font = font(tokens::TYPOGRAPHY_BODY.weight);
        let translation_size = px(tokens::TYPOGRAPHY_BODY.size);

        div()
            .flex()
            .flex_col()
            .gap(px(tokens::SPACE_3))
            .children(self.segments.iter().map(|segment| {
                let text_color = match segment.stage {
                    CaptionStage::Draft => colors.overlay_text_draft,
                    CaptionStage::Final => colors.overlay_text_primary,
                };
                let original = wrap(
                    &segment.original,
                    &caption_font,
                    caption_size,
                    self.width,
                    window,
                );
                let translation = segment.translation.as_deref().map(|translation| {
                    wrap(
                        translation,
                        &translation_font,
                        translation_size,
                        self.width,
                        window,
                    )
                });

                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .font(caption_font.clone())
                            .text_size(caption_size)
                            .line_height(caption_size * tokens::FONT_LINE_HEIGHT_TIGHT)
                            .text_color(color(text_color))
                            .children(original.into_iter().map(|line| div().child(line))),
                    )
                    .children(translation.map(|lines| {
                        div()
                            .font(translation_font.clone())
                            .text_size(translation_size)
                            .line_height(translation_size * tokens::FONT_LINE_HEIGHT_NORMAL)
                            .text_color(color(colors.overlay_text_draft))
                            .children(lines.into_iter().map(|line| div().child(line)))
                    }))
            }))
    }
}
