//! The design system in one window: every GPUI component in every variant.
//!
//!   cargo run --package malmoon-ui --example gallery [-- --light] [--exit-after <seconds>]

use std::time::Duration;

use gpui_kit::assets::Assets;
use gpui_kit::component::{Disableable, input::InputState};
use gpui_kit::{
    AppContext, Bounds, Context, Entity, IntoElement, ParentElement, Render, Styled, Window,
    WindowBounds, WindowOptions, div, px, size,
};
use malmoon_tokens as tokens;
use malmoon_ui::{
    Badge, BadgeTone, Banner, BannerTone, ButtonVariant, ControlSize, TextField, button, card,
    color, divider, palette, text,
};

struct Gallery {
    email: Entity<InputState>,
    code: Entity<InputState>,
    organization: Entity<InputState>,
}

impl Gallery {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut field = |placeholder: &'static str| {
            cx.new(|cx| InputState::new(window, cx).placeholder(placeholder))
        };
        Self {
            email: field("name@example.com"),
            code: field("000000"),
            organization: field("Malmoon"),
        }
    }
}

const VARIANTS: [(&str, ButtonVariant); 4] = [
    ("primary", ButtonVariant::Primary),
    ("secondary", ButtonVariant::Secondary),
    ("ghost", ButtonVariant::Ghost),
    ("danger", ButtonVariant::Danger),
];

const TEXT_STYLES: [(&str, tokens::Typography); 9] = [
    ("display", tokens::TYPOGRAPHY_DISPLAY),
    ("title", tokens::TYPOGRAPHY_TITLE),
    ("heading", tokens::TYPOGRAPHY_HEADING),
    ("body", tokens::TYPOGRAPHY_BODY),
    ("body-strong", tokens::TYPOGRAPHY_BODY_STRONG),
    ("label", tokens::TYPOGRAPHY_LABEL),
    ("detail", tokens::TYPOGRAPHY_DETAIL),
    ("footnote", tokens::TYPOGRAPHY_FOOTNOTE),
    ("caption", tokens::TYPOGRAPHY_CAPTION),
];

impl Render for Gallery {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = palette(cx);
        let row = || {
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap(px(tokens::SPACE_2))
        };
        let column = || div().flex_1().flex().flex_col().gap(px(tokens::SPACE_4));

        let texts =
            TEXT_STYLES.map(|(name, style)| text(format!("{name} · 회의 자막 Malmoon 123"), style));

        let buttons = VARIANTS.map(|(name, variant)| {
            row()
                .child(button((name, 0usize), "자막", variant, ControlSize::Sm))
                .child(button(
                    (name, 1usize),
                    "자막 시작",
                    variant,
                    ControlSize::Md,
                ))
                .child(button(
                    (name, 2usize),
                    "자막 시작",
                    variant,
                    ControlSize::Lg,
                ))
                .child(button((name, 3usize), "불가", variant, ControlSize::Md).disabled(true))
                .child(button((name, 4usize), "연결 중", variant, ControlSize::Md).loading(true))
        });

        let badges = row()
            .child(Badge::new("neutral"))
            .child(Badge::new("accent").tone(BadgeTone::Accent))
            .child(Badge::new("연결됨").tone(BadgeTone::Success).dot())
            .child(Badge::new("연결하는 중").tone(BadgeTone::Warning).dot())
            .child(Badge::new("연결 끊김").tone(BadgeTone::Danger).dot());

        let banners = [
            (BannerTone::Info, "info"),
            (BannerTone::Success, "success"),
            (BannerTone::Warning, "warning"),
            (BannerTone::Danger, "danger"),
        ]
        .map(|(tone, name)| Banner::new(tone, "자막은 이 기기에만 저장됩니다.").title(name));

        div()
            .size_full()
            .flex()
            .gap(px(tokens::SPACE_8))
            .p(px(tokens::SPACE_6))
            .bg(color(colors.background_base))
            .text_color(color(colors.text_primary))
            .child(column().children(texts).child(badges).children(banners))
            .child(
                column()
                    .children(buttons)
                    .child(
                        TextField::new("이메일", &self.email)
                            .description("로그인 코드를 보낼 주소입니다."),
                    )
                    .child(
                        TextField::new("로그인 코드", &self.code)
                            .error(Some("코드가 맞지 않거나 만료되었습니다.")),
                    )
                    .child(TextField::new("조직", &self.organization).disabled(true))
                    .child(
                        card(cx)
                            .child(text("실시간 자막", tokens::TYPOGRAPHY_HEADING))
                            .child(divider(cx))
                            .child(
                                text(
                                    "카드는 관련된 내용을 하나로 묶는다.",
                                    tokens::TYPOGRAPHY_BODY,
                                )
                                .text_color(color(colors.text_secondary)),
                            ),
                    ),
            )
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let light = args.iter().any(|arg| arg == "--light");
    let exit_after = args
        .iter()
        .position(|arg| arg == "--exit-after")
        .and_then(|index| args.get(index + 1))
        .and_then(|seconds| seconds.parse().ok())
        .map(Duration::from_secs_f64);

    gpui_kit::application().with_assets(Assets).run(move |cx| {
        malmoon_ui::init(cx);
        if light {
            malmoon_ui::set_mode(tokens::Mode::Light, cx);
        }
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(1120.0), px(760.0)),
                cx,
            ))),
            ..Default::default()
        };
        gpui_kit::open_window(options, cx, |window, cx| {
            cx.new(|cx| Gallery::new(window, cx))
        })
        .expect("the gallery window could not open");
        cx.activate(true);

        if let Some(exit_after) = exit_after {
            cx.spawn(async move |cx| {
                cx.background_executor().timer(exit_after).await;
                cx.update(|cx| cx.quit());
            })
            .detach();
        }
    });
}
