use gpui::{Context, FontWeight, div, prelude::*, px, svg};

use crate::main_window::MainWindow;
use crate::models::Page;
use crate::theme::{Theme, theme};
use crate::widgets::fade_in;

/// Horizontal inset of the sidebar column; the title bar text uses
/// `SIDEBAR_PAD + ITEM_PAD` so "Hush" lines up with item content.
pub(super) const SIDEBAR_PAD: f32 = 10.;
pub(super) const ITEM_PAD: f32 = 10.;

impl MainWindow {
    pub(super) fn render_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx);
        div()
            .flex()
            .flex_col()
            .justify_between()
            .w(px(192.))
            .h_full()
            .flex_none()
            .px(px(SIDEBAR_PAD))
            .pt(px(6.))
            .pb(px(10.))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(1.))
                    .child(self.nav_item(t, "Home", "icons/home.svg", Page::Home, cx))
                    .child(self.nav_item(t, "Dictionary", "icons/book.svg", Page::Dictionary, cx))
                    .child(self.nav_item(t, "Shortcuts", "icons/keyboard.svg", Page::Shortcuts, cx))
                    .child(self.nav_item(t, "Style", "icons/style.svg", Page::Style, cx)),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .children(self.update_button(t, cx))
                    .child(self.nav_item(t, "Settings", "icons/settings.svg", Page::Settings, cx)),
            )
    }

    /// Cream pill above Settings; only when a newer release exists.
    fn update_button(&self, t: Theme, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        let u = crate::update::updates(cx);
        u.available.as_ref()?;
        let downloading = u.downloading;
        let pill = div()
            .id("update-button")
            .flex()
            .items_center()
            .justify_center()
            .gap(px(7.))
            .h(px(34.))
            .rounded_full()
            .bg(t.primary)
            .text_color(t.primary_text)
            .text_size(px(13.))
            .font_weight(FontWeight::SEMIBOLD)
            .when(downloading, |el| el.opacity(0.7))
            .when(!downloading, |el| {
                el.hover(move |s| s.bg(t.primary_active))
                    .active(|s| s.opacity(0.88))
                    .cursor_pointer()
            })
            .child(
                svg()
                    .path("icons/download.svg")
                    .size(px(14.))
                    .flex_none()
                    .text_color(t.primary_text),
            )
            .child(if downloading { "Updating\u{2026}" } else { "Update now" })
            .on_click(|_, _, cx| crate::update::install(cx));
        Some(fade_in(pill, "update-in", 0, true))
    }

    fn nav_item(
        &self,
        t: Theme,
        label: &'static str,
        icon: &'static str,
        page: Page,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selected = self.page == page;
        let fg = if selected { t.text } else { t.text_secondary };
        div()
            .id(label)
            .group(label)
            .flex()
            .items_center()
            .gap(px(8.))
            .h(px(37.))
            .px(px(ITEM_PAD))
            .rounded(px(7.))
            .text_size(px(13.))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(fg)
            .when(selected, |el| el.bg(t.nav_selected))
            .when(!selected, |el| el.hover(move |s| s.bg(t.nav_hover).text_color(t.text)))
            .active(|s| s.opacity(0.9))
            .cursor_pointer()
            .child(
                svg()
                    .path(icon)
                    .size(px(16.))
                    .flex_none()
                    .text_color(fg)
                    .group_hover(label, move |s| s.text_color(t.text)),
            )
            .child(label)
            .on_click(cx.listener(move |this, _, _, cx| this.set_page(page, cx)))
    }
}
