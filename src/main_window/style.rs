//! Style page: how dictated text reads once it is typed.

use std::time::Duration;

use gpui::{Animation, AnimationExt, Context, FontWeight, div, prelude::*, px};

use crate::main_window::MainWindow;
use crate::main_window::pages::{page, page_header};
use crate::models::apply_style;
use crate::prefs::{Style, prefs, update_prefs};
use crate::theme::{RADIUS_PX, Theme, theme};
use crate::widgets::{ease_strong, selection_dot};

/// One sentence, as spoken, rendered in each style.
const SPOKEN: &str = "um so i think we should, uh, ship the build on Friday. what do you think?";

const STYLES: [(Style, &str, &str); 3] = [
    (Style::Formal, "Formal", "Full sentences, capitalized and punctuated. Fillers removed."),
    (Style::Casual, "Casual", "Reads like a text. Lowercase starts, fillers removed."),
    (Style::Verbatim, "Verbatim", "Exactly what you said, every um and uh kept."),
];

impl MainWindow {
    pub(super) fn render_style(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx);
        let current = prefs(cx).style;
        page()
            .child(page_header(t, "Style", "How your words read once they're typed"))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .pb(px(20.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(t.text_muted)
                            .child("You say"),
                    )
                    .child(
                        div()
                            .text_size(px(15.))
                            .text_color(t.text_secondary)
                            .child(format!("\u{201c}{SPOKEN}\u{201d}")),
                    ),
            )
            .child({
                let mut cards = Vec::new();
                for &(style, title, detail) in &STYLES {
                    cards.push(self.style_card(t, style, title, detail, style == current, cx));
                }
                div().w_full().grid().grid_cols(3).gap(px(12.)).children(cards)
            })
    }

    fn style_card(
        &self,
        t: Theme,
        style: Style,
        title: &'static str,
        detail: &'static str,
        selected: bool,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        div()
            .id(title)
            .relative()
            .min_w(px(0.))
            // Fixed height: GPUI grid rows don't size to content.
            .h(px(212.))
            .flex()
            .flex_col()
            .p(px(16.))
            .rounded(px(RADIUS_PX))
            .bg(t.surface)
            .border_1()
            .border_color(t.border)
            .when(!selected, |el| el.hover(move |s| s.border_color(t.border_strong)))
            .active(|s| s.opacity(0.9))
            .cursor_pointer()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(15.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(t.text)
                            .child(title),
                    )
                    .child(selection_dot(t, selected, self.style_nonce)),
            )
            .child(
                div()
                    .pt(px(4.))
                    .h(px(40.))
                    .text_size(px(12.))
                    .line_height(px(17.))
                    .text_color(t.text_muted)
                    .child(detail),
            )
            .child(
                // Inset preview: the spoken line as a sent message.
                div()
                    .mt(px(12.))
                    .p(px(10.))
                    .h(px(96.))
                    .rounded(px(10.))
                    .bg(t.track)
                    .flex()
                    .flex_col()
                    .justify_end()
                    .items_end()
                    .overflow_hidden()
                    .child(
                        div()
                            .max_w_full()
                            .px(px(10.))
                            .py(px(7.))
                            .rounded(px(14.))
                            .rounded_br(px(4.))
                            .bg(t.primary)
                            .text_color(t.primary_text)
                            .text_size(px(12.))
                            .line_height(px(17.))
                            .child(apply_style(SPOKEN, style)),
                    ),
            )
            .when(selected, |el| {
                // Ink ring fades in over the hairline border.
                el.child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .right_0()
                        .bottom_0()
                        .rounded(px(RADIUS_PX))
                        .border_2()
                        .border_color(t.primary)
                        .with_animation(
                            ("style-ring", self.style_nonce),
                            Animation::new(Duration::from_millis(150)).with_easing(ease_strong),
                            |el, d| el.opacity(d),
                        ),
                )
            })
            .on_click(cx.listener(move |this, _, _, cx| {
                if prefs(cx).style != style {
                    this.style_nonce += 1;
                    update_prefs(cx, |p| p.style = style);
                }
            }))
            .into_any_element()
    }
}
