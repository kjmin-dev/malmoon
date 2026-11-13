//! The Malmoon desktop app.
//!
//!   malmoon [--click-through] [--hide-from-capture] [--exit-after <seconds>]
//!
//! `MALMOON_GATEWAY_URL` sets the gateway to connect to (default `ws://127.0.0.1:8787`), and
//! `MALMOON_WEB_URL` the web app that signs the person in (default `http://localhost:5173`).

mod auth;
mod platform;
mod prompter;
mod session;

use std::time::Duration;

use gpui_kit::assets::Assets;
use gpui_kit::{
    AppContext, Bounds, KeyBinding, WindowBackgroundAppearance, WindowBounds, WindowKind,
    WindowOptions, actions, px, size,
};

use crate::platform::OverlayOptions;
use crate::prompter::Prompter;

actions!(malmoon, [Quit]);

struct Options {
    session: session::Config,
    overlay: OverlayOptions,
    /// Quit after this long. For smoke tests.
    exit_after: Option<Duration>,
}

impl Options {
    fn from_env() -> Self {
        let args: Vec<String> = std::env::args().skip(1).collect();
        let has = |flag: &str| args.iter().any(|arg| arg == flag);
        let exit_after = args
            .iter()
            .position(|arg| arg == "--exit-after")
            .and_then(|index| args.get(index + 1))
            .and_then(|seconds| seconds.parse().ok())
            .map(Duration::from_secs_f64);
        Self {
            session: session::Config {
                gateway_url: std::env::var("MALMOON_GATEWAY_URL")
                    .unwrap_or_else(|_| "ws://127.0.0.1:8787".to_owned()),
                web_url: std::env::var("MALMOON_WEB_URL")
                    .unwrap_or_else(|_| "http://localhost:5173".to_owned()),
            },
            overlay: OverlayOptions {
                click_through: has("--click-through"),
                hide_from_capture: has("--hide-from-capture"),
            },
            exit_after,
        }
    }
}

fn main() {
    let options = Options::from_env();
    let session = session::spawn(options.session);

    gpui_kit::application().with_assets(Assets).run(move |cx| {
        malmoon_ui::init(cx);
        cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);
        cx.on_action(|_: &Quit, cx| cx.quit());

        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(720.0), px(360.0)),
                cx,
            ))),
            titlebar: None,
            // A pop-up floats above full-screen apps on every Space and does not take focus.
            kind: WindowKind::PopUp,
            focus: false,
            is_resizable: false,
            window_background: WindowBackgroundAppearance::Blurred,
            ..Default::default()
        };
        gpui_kit::open_window(window_options, cx, move |window, cx| {
            platform::configure_overlay(window, options.overlay);
            cx.new(|cx| Prompter::new(session, window, cx))
        })
        .expect("the prompter window could not open");

        if let Some(exit_after) = options.exit_after {
            cx.spawn(async move |cx| {
                cx.background_executor().timer(exit_after).await;
                cx.update(|cx| cx.quit());
            })
            .detach();
        }
    });
}
