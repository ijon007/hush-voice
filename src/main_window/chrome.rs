use gpui::{FontWeight, WindowControlArea, div, prelude::*, px};

use crate::main_window::MainWindow;
use crate::theme::{BG, BORDER, TEXT};
use crate::widgets::window_btn;

impl MainWindow {
    pub(super) fn render_titlebar(&self) -> impl IntoElement {
        div()
            .id("titlebar")
            .flex()
            .items_center()
            .w_full()
            .h(px(36.))
            .flex_none()
            .bg(BG)
            .border_b_1()
            .border_color(BORDER)
            .child(
                div()
                    .id("titlebar-drag")
                    .flex_1()
                    .h_full()
                    .flex()
                    .items_center()
                    .pl(px(16.))
                    .window_control_area(WindowControlArea::Drag)
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(TEXT)
                            .child("Hush"),
                    ),
            )
            .child(window_btn("win-min", "\u{2014}", |window, _| {
                window.minimize_window();
            }))
            .child(window_btn("win-close", "\u{2715}", |window, _| {
                window.remove_window();
            }))
    }
}
