use gpui::{Context, FontWeight, WindowControlArea, div, img, prelude::*, px};

use crate::main_window::MainWindow;
use crate::main_window::sidebar::{ITEM_PAD, SIDEBAR_PAD};
use crate::theme::theme;
use crate::widgets::window_btn;

impl MainWindow {
    pub(super) fn render_titlebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx);
        div()
            .id("titlebar")
            .flex()
            .items_center()
            .w_full()
            .h(px(36.))
            .flex_none()
            .bg(t.bg)
            .child(
                div()
                    .id("titlebar-drag")
                    .flex_1()
                    .h_full()
                    .flex()
                    .items_center()
                    .pl(px(SIDEBAR_PAD + ITEM_PAD))
                    .gap(px(7.))
                    .window_control_area(WindowControlArea::Drag)
                    .child(
                        // App icon (dark tile) in full color, same in both themes.
                        img("logo/icon.svg").size(px(16.)).flex_none(),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(t.text)
                            .child("Hush"),
                    ),
            )
            .child(window_btn(t, "win-min", "\u{2014}", |window, _| {
                window.minimize_window();
            }))
            .child(window_btn(t, "win-close", "\u{2715}", |window, _| {
                window.remove_window();
            }))
    }
}
