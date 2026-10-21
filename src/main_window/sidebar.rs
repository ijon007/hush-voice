use gpui::{Context, FontWeight, div, prelude::*, px, transparent_black};

use crate::main_window::MainWindow;
use crate::models::Page;
use crate::theme::{RADIUS_SM_PX, Theme, theme};

impl MainWindow {
    pub(super) fn render_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx);
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
            .bg(t.sidebar)
            .border_r_1()
            .border_color(t.border)
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
                            .text_color(t.text_muted)
                            .child("MENU"),
                    )
                    .child(self.nav_item(t, "Home", Page::Home, cx))
                    .child(self.nav_item(t, "Dictionary", Page::Dictionary, cx))
                    .child(self.nav_item(t, "Shortcuts", Page::Shortcuts, cx)),
            )
            .child(self.nav_item(t, "Settings", Page::Settings, cx))
    }

    fn nav_item(
        &self,
        t: Theme,
        label: &'static str,
        page: Page,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selected = self.page == page;
        div()
            .id(label)
            .flex()
            .items_center()
            .h(px(34.))
            .px(px(12.))
            .rounded(px(RADIUS_SM_PX))
            .border_1()
            .text_size(px(13.))
            .font_weight(if selected { FontWeight::MEDIUM } else { FontWeight::NORMAL })
            .text_color(if selected { t.text } else { t.text_secondary })
            .when(selected, |el| el.bg(t.surface).border_color(t.border).shadow_sm())
            .when(!selected, |el| el.border_color(transparent_black()))
            .hover(move |s| if selected { s } else { s.bg(t.surface_hover).text_color(t.text) })
            .active(|s| s.opacity(0.85))
            .cursor_pointer()
            .child(label)
            .on_click(cx.listener(move |this, _, _, cx| this.set_page(page, cx)))
    }
}
