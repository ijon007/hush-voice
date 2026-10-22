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

    /// Pale, softly tinted card above Settings; only when a newer release exists.
    fn update_button(&self, t: Theme, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        let u = crate::update::updates(cx);
        let release = u.available.as_ref()?;
        let downloading = u.downloading;
        let permille = u.progress.load(std::sync::atomic::Ordering::Relaxed);
        let tint = |a: f32| gpui::Rgba { a, ..t.primary };
        let action: gpui::SharedString = if downloading {
            if permille > 0 {
                format!("Downloading\u{2026} {}%", permille / 10).into()
            } else {
                "Downloading\u{2026}".into()
            }
        } else {
            format!("Update now \u{00b7} v{}", release.version).into()
        };
        let card = div()
            .id("update-button")
            .relative()
            .flex()
            .flex_col()
            .gap(px(1.))
            .px(px(ITEM_PAD + 2.))
            .py(px(9.))
            .rounded(px(10.))
            .bg(tint(0.06))
            .border_1()
            .border_color(tint(0.10))
            .overflow_hidden()
            .when(!downloading, |el| {
                el.hover(move |s| s.bg(tint(0.10)).border_color(tint(0.16)))
                    .active(|s| s.opacity(0.85))
                    .cursor_pointer()
            })
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .text_size(px(11.))
                    .text_color(t.text_muted)
                    .child(div().size(px(5.)).rounded_full().bg(tint(0.55)))
                    .child("New update available"),
            )
            .child(
                div()
                    .text_size(px(13.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(t.text)
                    .child(action),
            )
            .when(downloading, |el| {
                // Hairline progress along the bottom edge.
                el.child(
                    div()
                        .absolute()
                        .left_0()
                        .bottom_0()
                        .h(px(2.))
                        .w(gpui::relative(permille as f32 / 1000.))
                        .bg(tint(0.35)),
                )
            })
            .on_click(|_, _, cx| crate::update::install(cx));
        Some(fade_in(card, "update-in", 0, true))
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
