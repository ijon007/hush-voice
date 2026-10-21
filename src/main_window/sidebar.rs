use gpui::{Context, FontWeight, div, prelude::*, px};

use crate::main_window::MainWindow;
use crate::models::Page;
use crate::theme::{BG, BORDER, RADIUS_SM_PX, SURFACE, SURFACE_HOVER, TEXT, TEXT_MUTED, TEXT_SECONDARY};

impl MainWindow {
    pub(super) fn render_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .justify_between()
            .w(px(216.))
            .h_full()
            .flex_none()
            .px(px(12.))
            .pt(px(16.))
            .pb(px(16.))
            .bg(BG)
            .border_r_1()
            .border_color(BORDER)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .child(
                        div()
                            .px(px(12.))
                            .pb(px(8.))
                            .text_size(px(11.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(TEXT_MUTED)
                            .child("MENU"),
                    )
                    .child(self.nav_item("Home", Page::Home, cx))
                    .child(self.nav_item("Dictionary", Page::Dictionary, cx))
                    .child(self.nav_item("Shortcuts", Page::Shortcuts, cx)),
            )
            .child(self.nav_item("Settings", Page::Settings, cx))
    }

    fn nav_item(&self, label: &'static str, page: Page, cx: &mut Context<Self>) -> impl IntoElement {
        let selected = self.page == page;
        div()
            .id(label)
            .flex()
            .items_center()
            .h(px(34.))
            .px(px(12.))
            .rounded(px(RADIUS_SM_PX))
            .text_size(px(13.))
            .font_weight(if selected { FontWeight::MEDIUM } else { FontWeight::NORMAL })
            .text_color(if selected { TEXT } else { TEXT_SECONDARY })
            .when(selected, |el| el.bg(SURFACE).border_1().border_color(BORDER).shadow_sm())
            .when(!selected, |el| el.border_1().border_color(gpui::transparent_black()))
            .hover(move |s| if selected { s } else { s.bg(SURFACE_HOVER).text_color(TEXT) })
            .active(|s| s.opacity(0.85))
            .cursor_pointer()
            .child(label)
            .on_click(cx.listener(move |this, _, _, cx| this.set_page(page, cx)))
    }
}
