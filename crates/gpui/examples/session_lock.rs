#![cfg_attr(target_family = "wasm", no_main)]

fn run_example() {
    #[cfg(all(target_os = "linux", feature = "wayland"))]
    example::main();

    #[cfg(not(all(target_os = "linux", feature = "wayland")))]
    panic!("This example requires the `wayland` feature and a linux system.");
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    run_example();
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    gpui_platform::web_init();
    run_example();
}

#[cfg(all(target_os = "linux", feature = "wayland"))]
mod example {
    use std::time::Duration;

    use gpui::{App, Context, FontWeight, Render, Window, div, prelude::*, rems, rgba, white};
    use gpui_platform::application;

    struct SessionLockExample {
        remaining: u32,
    }

    impl SessionLockExample {
        fn new(cx: &mut Context<Self>) -> Self {
            cx.spawn(async move |this, cx| {
                for _ in 0..5 {
                    cx.background_executor().timer(Duration::from_secs(1)).await;
                    this.update(cx, |this, cx| {
                        this.remaining = this.remaining.saturating_sub(1);
                        cx.notify();
                    })
                    .ok();
                }
            })
            .detach();

            SessionLockExample { remaining: 5 }
        }
    }

    impl Render for SessionLockExample {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .size_full()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .bg(rgba(0x222222ff))
                .child(
                    div()
                        .text_size(rems(4.5))
                        .font_weight(FontWeight::EXTRA_BOLD)
                        .text_color(white())
                        .child("Session Locked"),
                )
                .child(
                    div()
                        .text_size(rems(2.0))
                        .text_color(rgba(0xaaaaaaff))
                        .child(format!("Unlocking in {}s...", self.remaining)),
                )
        }
    }

    pub fn main() {
        application().run(|cx: &mut App| {
            // unlock after 5s
            cx.spawn(async move |cx| {
                cx.background_executor().timer(Duration::from_secs(5)).await;
                cx.update(|cx| {
                    cx.unlock_session().ok();
                });
            })
            .detach();

            cx.lock_session(|_window, cx| cx.new(SessionLockExample::new))
                .expect("Failed to lock session");
        });
    }
}
