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
    use gpui::{
        App, Bounds, BoundsHandle, Context, FocusHandle, Focusable, KeyBinding, Size, Window,
        WindowBounds, WindowKind, WindowOptions, actions, div, layer_shell::*, point, popup::*,
        prelude::*, px, rgb, size,
    };
    use gpui_platform::application;

    actions!(
        xdg_popup_example,
        [Quit, OpenPopup, SelectPrev, SelectNext, Confirm]
    );

    const MENU_ITEMS: &[(&str, &str)] = &[
        ("Cut", "Ctrl+X"),
        ("Copy", "Ctrl+C"),
        ("Paste", "Ctrl+V"),
        ("Select All", "Ctrl+A"),
        ("Find...", "Ctrl+F"),
        ("Replace...", "Ctrl+H"),
    ];

    struct PopupContent {
        selected: usize,
        focus_handle: FocusHandle,
    }

    impl Focusable for PopupContent {
        fn focus_handle(&self, _: &App) -> FocusHandle {
            self.focus_handle.clone()
        }
    }

    impl Render for PopupContent {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let selected = self.selected;
            let focused = self.focus_handle.is_focused(window);
            let active = window.is_window_active();
            div()
                .track_focus(&self.focus_handle)
                .on_action(cx.listener(|this, _: &SelectPrev, _, cx| {
                    eprintln!("[popup] SelectPrev");
                    this.selected = if this.selected == 0 {
                        MENU_ITEMS.len() - 1
                    } else {
                        this.selected - 1
                    };
                    cx.notify();
                }))
                .on_action(cx.listener(|this, _: &SelectNext, _, cx| {
                    eprintln!("[popup] SelectNext");
                    this.selected = (this.selected + 1) % MENU_ITEMS.len();
                    cx.notify();
                }))
                .on_action(cx.listener(|this, _: &Confirm, _, cx| {
                    let (label, _) = MENU_ITEMS[this.selected];
                    eprintln!("[popup] Confirm: {label}");
                    cx.notify();
                }))
                .on_action(cx.listener(|_, _: &Quit, window, _| {
                    eprintln!("[popup] Quit -> close");
                    window.remove_window();
                }))
                .size_full()
                .flex()
                .flex_col()
                .bg(rgb(0x2d2d2d))
                .text_color(rgb(0xffffff))
                .child(focus_badge("popup", focused, active))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .p_1()
                        .child(menu_item(MENU_ITEMS[0].0, MENU_ITEMS[0].1, selected == 0))
                        .child(menu_item(MENU_ITEMS[1].0, MENU_ITEMS[1].1, selected == 1))
                        .child(menu_item(MENU_ITEMS[2].0, MENU_ITEMS[2].1, selected == 2))
                        .child(div().h(px(1.)).mx_1().my(px(2.)).bg(rgb(0x555555)))
                        .child(menu_item(MENU_ITEMS[3].0, MENU_ITEMS[3].1, selected == 3))
                        .child(menu_item(MENU_ITEMS[4].0, MENU_ITEMS[4].1, selected == 4))
                        .child(menu_item(MENU_ITEMS[5].0, MENU_ITEMS[5].1, selected == 5)),
                )
        }
    }

    fn menu_item(label: &str, shortcut: &str, selected: bool) -> impl IntoElement {
        div()
            .id(label.to_string())
            .flex()
            .justify_between()
            .px_3()
            .py_1()
            .rounded_sm()
            .cursor_pointer()
            .when(selected, |style| style.bg(rgb(0x094771)))
            .hover(|style| style.bg(rgb(0x094771)))
            .child(label.to_string())
            .child(
                div()
                    .text_color(rgb(0x888888))
                    .text_size(px(12.))
                    .child(shortcut.to_string()),
            )
    }

    fn focus_badge(name: &str, gpui_focused: bool, wayland_active: bool) -> impl IntoElement {
        let bg = match (gpui_focused, wayland_active) {
            (true, true) => rgb(0x1e6633),
            (true, false) | (false, true) => rgb(0x665533),
            (false, false) => rgb(0x663333),
        };
        div()
            .px_3()
            .py_1()
            .text_size(px(11.))
            .text_color(rgb(0xffffff))
            .bg(bg)
            .child(format!(
                "{name}: gpui focus = {}, wayland active = {}",
                gpui_focused, wayland_active
            ))
    }

    fn popup_button(label: &str) -> gpui::Stateful<gpui::Div> {
        div()
            .id(label.to_string())
            .flex_none()
            .px_3()
            .py_1()
            .bg(rgb(0x0078d4))
            .text_color(rgb(0xffffff))
            .rounded_md()
            .cursor_pointer()
            .hover(|style| style.bg(rgb(0x106ebe)))
            .active(|style| style.bg(rgb(0x005a9e)))
            .child(label.to_string())
    }

    struct PopupDemo {
        anchor_bounds: BoundsHandle,
        focus_handle: FocusHandle,
    }

    impl Focusable for PopupDemo {
        fn focus_handle(&self, _: &App) -> FocusHandle {
            self.focus_handle.clone()
        }
    }

    impl PopupDemo {
        fn open_popup(
            &self,
            anchor: PopupAnchor,
            gravity: PopupGravity,
            _window: &mut Window,
            cx: &mut App,
        ) {
            let bounds = self.anchor_bounds.bounds();
            if bounds.size.width == px(0.) || bounds.size.height == px(0.) {
                return;
            }

            match cx.open_window(
                WindowOptions {
                    titlebar: None,
                    window_bounds: Some(WindowBounds::Windowed(Bounds {
                        origin: point(px(0.), px(0.)),
                        size: Size::new(px(200.), px(220.)),
                    })),
                    kind: WindowKind::XdgPopup(PopupOptions {
                        anchor_rect: bounds,
                        anchor,
                        gravity,
                        constraint_adjustment: PopupConstraintAdjustment::FLIP_X
                            | PopupConstraintAdjustment::FLIP_Y
                            | PopupConstraintAdjustment::SLIDE_X
                            | PopupConstraintAdjustment::SLIDE_Y,
                        offset: None,
                        reactive: true,
                    }),
                    ..Default::default()
                },
                |_, cx| {
                    cx.new(|cx| PopupContent {
                        selected: 0,
                        focus_handle: cx.focus_handle(),
                    })
                },
            ) {
                Ok(window) => {
                    window
                        .update(cx, |view, window, cx| {
                            window.focus(&view.focus_handle(cx), cx);
                        })
                        .ok();
                }
                Err(error) => eprintln!("Failed to open popup: {error}"),
            }
        }
    }

    impl Render for PopupDemo {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let focused = self.focus_handle.is_focused(window);
            let active = window.is_window_active();
            div()
                .track_focus(&self.focus_handle)
                .on_action(cx.listener(|this, _: &OpenPopup, window, cx| {
                    eprintln!("[parent] OpenPopup");
                    this.open_popup(PopupAnchor::Bottom, PopupGravity::Bottom, window, cx);
                }))
                .flex()
                .flex_col()
                .bg(rgb(0xf0f0f0))
                .size_full()
                .p_4()
                .gap_4()
                .child(
                    div()
                        .text_size(px(18.))
                        .font_weight(gpui::FontWeight::BOLD)
                        .child("xdg_popup Example"),
                )
                .child(focus_badge("parent", focused, active))
                .child(div().text_size(px(13.)).text_color(rgb(0x555555)).child(
                    "Click a button or press space to open a popup menu anchored to the \
                             target rectangle below. In the popup: up/down navigate, enter \
                             confirms, escape closes the popup. Outside the popup, escape \
                             quits.",
                ))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap_3()
                        .child(
                            div()
                                .flex()
                                .gap_2()
                                .child(popup_button("Below (default)").on_click(cx.listener(
                                    |this, _, window, cx| {
                                        this.open_popup(
                                            PopupAnchor::Bottom,
                                            PopupGravity::Bottom,
                                            window,
                                            cx,
                                        );
                                    },
                                )))
                                .child(popup_button("Above").on_click(cx.listener(
                                    |this, _, window, cx| {
                                        this.open_popup(
                                            PopupAnchor::Top,
                                            PopupGravity::Top,
                                            window,
                                            cx,
                                        );
                                    },
                                )))
                                .child(popup_button("Right").on_click(cx.listener(
                                    |this, _, window, cx| {
                                        this.open_popup(
                                            PopupAnchor::Right,
                                            PopupGravity::Right,
                                            window,
                                            cx,
                                        );
                                    },
                                )))
                                .child(popup_button("Left").on_click(cx.listener(
                                    |this, _, window, cx| {
                                        this.open_popup(
                                            PopupAnchor::Left,
                                            PopupGravity::Left,
                                            window,
                                            cx,
                                        );
                                    },
                                ))),
                        )
                        .child(
                            div()
                                .id("anchor-target")
                                .track_bounds(&self.anchor_bounds)
                                .flex()
                                .items_center()
                                .justify_center()
                                .w(px(200.))
                                .h(px(60.))
                                .bg(rgb(0xd0d0d0))
                                .border_2()
                                .border_color(rgb(0x0078d4))
                                .rounded_md()
                                .child("Anchor Target"),
                        ),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(rgb(0x888888))
                        .child("Popups use xdg_positioner for compositor-managed placement."),
                )
        }
    }

    pub fn main() {
        application().run(|cx: &mut App| {
            let bounds = Bounds::centered(None, size(px(500.), px(400.)), cx);

            let window = cx
                .open_window(
                    WindowOptions {
                        titlebar: None,
                        window_bounds: Some(WindowBounds::Windowed(bounds)),
                        app_id: Some("gpui-xdg-popup-example".to_string()),
                        kind: WindowKind::LayerShell(LayerShellOptions {
                            namespace: "gpui-xdg-popup-example".to_string(),
                            keyboard_interactivity: KeyboardInteractivity::OnDemand,
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                    |_, cx| {
                        cx.new(|cx| PopupDemo {
                            anchor_bounds: BoundsHandle::new(),
                            focus_handle: cx.focus_handle(),
                        })
                    },
                )
                .unwrap();

            window
                .update(cx, |view, window, cx| {
                    window.focus(&view.focus_handle(cx), cx);
                })
                .unwrap();

            cx.activate(true);
            cx.on_action(|_: &Quit, cx| {
                eprintln!("[global] Quit");
                cx.quit();
            });
            cx.bind_keys([
                KeyBinding::new("escape", Quit, None),
                KeyBinding::new("space", OpenPopup, None),
                KeyBinding::new("up", SelectPrev, None),
                KeyBinding::new("down", SelectNext, None),
                KeyBinding::new("enter", Confirm, None),
            ]);
        });
    }
}
