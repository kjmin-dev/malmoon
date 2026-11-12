//! The prompter: live captions over the meeting, and a bar to ask or translate.

use futures_channel::mpsc::UnboundedReceiver;
use futures_util::StreamExt;
use gpui_kit::component::{
    Sizable,
    button::{Button, ButtonVariants},
    input::{Input, InputEvent, InputState},
};
use gpui_kit::{
    AppContext, Context, Entity, IntoElement, ParentElement, Render, SharedString, Styled,
    Subscription, Window, div, px,
};
use malmoon_core::caption::CaptionBuffer;
use malmoon_tokens as tokens;
use malmoon_ui::{CaptionStream, color};

use crate::session::SessionEvent;

/// Settled segments kept on screen. Older ones scroll away.
const VISIBLE_SEGMENTS: usize = 3;

enum Status {
    Connecting,
    Live,
    Offline(String),
}

pub struct Prompter {
    captions: CaptionBuffer,
    status: Status,
    ask: Entity<InputState>,
    _subscriptions: Vec<Subscription>,
}

impl Prompter {
    pub fn new(
        mut events: UnboundedReceiver<SessionEvent>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let ask = cx.new(|cx| {
            InputState::new(window, cx).placeholder("질문하거나 번역할 문장을 입력하세요")
        });
        let activate_on_focus = cx.subscribe_in(&ask, window, |_, _, event: &InputEvent, _, cx| {
            // The overlay never takes focus from the meeting, but macOS input methods only
            // compose (Hangul, Kana, Pinyin) while this app is the active one. Typing here
            // is the one moment the user wants the keyboard, so take it.
            if matches!(event, InputEvent::Focus) {
                cx.activate(true);
            }
        });

        cx.spawn(async move |this, cx| {
            while let Some(event) = events.next().await {
                let updated = this.update(cx, |this, cx| {
                    this.handle(event);
                    cx.notify();
                });
                if updated.is_err() {
                    break;
                }
            }
        })
        .detach();

        Self {
            captions: CaptionBuffer::new(VISIBLE_SEGMENTS),
            status: Status::Connecting,
            ask,
            _subscriptions: vec![activate_on_focus],
        }
    }

    fn handle(&mut self, event: SessionEvent) {
        match event {
            SessionEvent::Connecting => self.status = Status::Connecting,
            SessionEvent::Ready => {
                self.status = Status::Live;
                // Segment ids restart with every session.
                self.captions.clear();
            }
            SessionEvent::Caption(caption) => {
                self.captions.apply(caption);
            }
            SessionEvent::Disconnected { reason } => self.status = Status::Offline(reason),
        }
    }
}

impl Render for Prompter {
    fn render(&mut self, window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let colors = tokens::colors(tokens::Mode::Dark);
        let padding = px(tokens::SPACE_4);
        let text_width = window.viewport_size().width - padding * 2.0;

        let (status_color, status_text): (_, SharedString) = match &self.status {
            Status::Connecting => (colors.status_warning, "게이트웨이에 연결하는 중".into()),
            Status::Live => (colors.status_success, "연결됨".into()),
            Status::Offline(reason) => {
                (colors.status_danger, format!("연결 끊김 · {reason}").into())
            }
        };

        let header = div()
            .flex()
            .items_center()
            .gap(px(tokens::SPACE_2))
            .text_size(px(tokens::FONT_SIZE_XS))
            .text_color(color(colors.overlay_text_draft))
            .child(
                div()
                    .flex_none()
                    .size(px(tokens::SPACE_2))
                    .rounded_full()
                    .bg(color(status_color)),
            )
            .child(div().flex_1().truncate().child(status_text))
            .child(
                Button::new("quit")
                    .ghost()
                    .xsmall()
                    .label("종료")
                    .on_click(|_, _, cx| cx.quit()),
            );

        let captions = if self.captions.is_empty() {
            let hint = match self.status {
                Status::Offline(_) => "`mise run //apps/gateway:dev`로 게이트웨이를 실행하세요.",
                _ => "자막을 기다리는 중",
            };
            div()
                .text_color(color(colors.overlay_text_draft))
                .child(hint)
                .into_any_element()
        } else {
            CaptionStream::new(self.captions.segments(), text_width).into_any_element()
        };

        div()
            .size_full()
            .flex()
            .flex_col()
            .gap(px(tokens::SPACE_3))
            .p(padding)
            .bg(color(colors.overlay_surface))
            .text_color(color(colors.overlay_text_primary))
            .child(header)
            // New captions appear at the bottom and push older ones up and out of view.
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .justify_end()
                    .overflow_hidden()
                    .child(captions),
            )
            .child(Input::new(&self.ask))
    }
}
