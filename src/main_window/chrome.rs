use gpui::{Context, FontWeight, WindowControlArea, div, prelude::*, px};

use crate::main_window::MainWindow;
use crate::models::Tab;
use crate::theme::{
    BORDER, RADIUS_PX, SURFACE, SURFACE_ACTIVE, SURFACE_HOVER, TEXT, TEXT_MUTED, TEXT_SECONDARY,
};
use crate::widgets::window_btn;

impl MainWindow {
    pub(super) fn render_titlebar(&self) -> impl IntoElement {
        div()
            .id("titlebar")
            .flex()
            .items_center()
            .w_full()
            .h(px(36.))
            .border_b_1()
            .border_color(BORDER)
            .child(
                div()
                    .id("titlebar-drag")
                    .flex_1()
                    .h_full()
                    .flex()
                    .items_center()
                    .pl(px(14.))
                    .window_control_area(WindowControlArea::Drag)
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(TEXT)
                            .child("Hush"),
                    ),
            )
            .child(window_btn("win-close", "✕", |window, _| {
                window.remove_window();
            }))
    }

    pub(super) fn render_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .justify_between()
            .w_full()
            .px(px(24.))
            .pt(px(14.))
            .pb(px(14.))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .child(
                        div()
                            .text_size(px(20.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Welcome back!"),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(TEXT_MUTED)
                            .child("Hold Ctrl + Space · Ctrl + Alt + Space toggles"),
                    ),
            )
            .child(self.render_tabs(cx))
    }

    fn render_tabs(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .p(px(3.))
            .gap(px(2.))
            .rounded(px(RADIUS_PX))
            .bg(SURFACE)
            .border_1()
            .border_color(BORDER)
            .child(self.tab_button("History", Tab::History, cx))
            .child(self.tab_button("Settings", Tab::Settings, cx))
    }

    fn tab_button(
        &self,
        label: &'static str,
        tab: Tab,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selected = self.tab == tab;
        div()
            .id(label)
            .px(px(12.))
            .py(px(6.))
            .rounded(px(RADIUS_PX - 2.))
            .text_size(px(13.))
            .font_weight(if selected {
                FontWeight::MEDIUM
            } else {
                FontWeight::NORMAL
            })
            .text_color(if selected { TEXT } else { TEXT_SECONDARY })
            .when(selected, |el| el.bg(SURFACE_HOVER))
            .hover(|s| {
                if selected {
                    s
                } else {
                    s.bg(SURFACE_ACTIVE).text_color(TEXT)
                }
            })
            .active(|s| s.opacity(0.85))
            .cursor_pointer()
            .child(label)
            .on_click(cx.listener(move |this, _, _, cx| this.set_tab(tab, cx)))
    }
}
