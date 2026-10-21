use std::time::Duration;

use gpui::{
    Animation, AnimationExt, Context, FontWeight, SharedString, div, ease_out_quint, prelude::*,
    px, svg,
};

use crate::main_window::MainWindow;
use crate::models::{TranscriptRow, compute_stats};
use crate::theme::{RADIUS_PX, Theme, theme};
use crate::widgets::{card, empty_state, fade_in, section_label};

pub(super) fn page_header(t: Theme, title: &'static str, subtitle: &'static str) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap(px(4.))
        .pb(px(24.))
        .child(
            div()
                .text_size(px(26.))
                .font_weight(FontWeight::SEMIBOLD)
                .line_height(px(32.))
                .text_color(t.text)
                .child(title),
        )
        .child(div().text_size(px(13.)).text_color(t.text_muted).child(subtitle))
}

fn page() -> gpui::Div {
    div().flex().flex_col().w_full()
}

impl MainWindow {
    pub(super) fn render_home(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx);
        page()
            .child(page_header(t, "Welcome back!", "Hold Ctrl + Space to dictate"))
            .child(self.render_stats(t))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(section_label(t, "History"))
                    .child(self.render_history(t, cx)),
            )
    }

    pub(super) fn render_settings_page(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx);
        page()
            .child(page_header(t, "Settings", "Model, hotkey and appearance"))
            .child(self.render_settings(cx))
    }

    pub(super) fn render_dictionary(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx);
        page()
            .child(page_header(t, "Dictionary", "Custom words and spellings"))
            .child(empty_state(t, "Nothing here yet", "Your custom vocabulary will live here"))
    }

    pub(super) fn render_shortcuts(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx);
        page()
            .child(page_header(t, "Shortcuts", "Keyboard shortcuts and snippets"))
            .child(empty_state(t, "Nothing here yet", "Your shortcuts will live here"))
    }
}

impl MainWindow {
    /// Grouped transcript list: one surface, hairline dividers, copy on hover.
    fn render_history(&self, t: Theme, cx: &mut Context<Self>) -> gpui::AnyElement {
        if self.history.is_empty() {
            return empty_state(t, "No transcripts yet", "Hold Ctrl + Space to capture the first one")
                .into_any_element();
        }
        let last = self.history.len() - 1;
        let mut rows = Vec::new();
        for (i, row) in self.history.iter().enumerate() {
            let el = self.history_row(t, i, row, i == last, cx);
            // First show only: rows fade in after the stats, 25ms apart (first 8).
            rows.push(if self.nav_count == 0 && i < 8 {
                fade_in(el, ("history-in", i), 120 + i as u64 * 25, false)
            } else {
                el.into_any_element()
            });
        }
        card(t).children(rows).into_any_element()
    }

    fn history_row(
        &self,
        t: Theme,
        index: usize,
        row: &TranscriptRow,
        is_last: bool,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let group: SharedString = format!("history-row-{index}").into();
        let copied = self.copied == Some(index);
        div()
            .id(("history-row", index))
            .group(group.clone())
            .flex()
            .items_center()
            .gap(px(12.))
            .pl(px(16.))
            .pr(px(10.))
            .py(px(14.))
            .hover(move |s| s.bg(t.surface_hover))
            .when(index == 0, |el| el.rounded_t(px(RADIUS_PX - 1.)))
            .when(is_last, |el| el.rounded_b(px(RADIUS_PX - 1.)))
            .when(!is_last, |el| el.border_b_1().border_color(t.border))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w(px(0.))
                    .gap(px(4.))
                    .child(
                        div()
                            .text_size(px(14.))
                            .text_color(t.text)
                            .line_height(px(20.))
                            .child(row.text.clone()),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .text_size(px(12.))
                            .text_color(t.text_muted)
                            .child(row.model.clone())
                            .child(div().text_color(t.border_strong).child("\u{00b7}"))
                            .child(row.duration_label()),
                    ),
            )
            .child(
                // Slot is always reserved; the button fades in on row hover.
                div()
                    .id(("copy", index))
                    .group(SharedString::from(format!("copy-{index}")))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(px(30.))
                    .rounded(px(7.))
                    .when(!copied, |el| el.opacity(0.))
                    .group_hover(group, |s| s.opacity(1.))
                    .hover(move |s| s.bg(t.surface_selected))
                    .active(move |s| s.bg(t.surface_active))
                    .cursor_pointer()
                    .child(if copied {
                        // Check pops in: scale 10px -> 15px with a fade.
                        svg()
                            .path("icons/check.svg")
                            .size(px(15.))
                            .text_color(t.text)
                            .with_animation(
                                ("copied", self.copy_nonce),
                                Animation::new(Duration::from_millis(180))
                                    .with_easing(ease_out_quint()),
                                |el, d| el.size(px(10. + 5. * d)).opacity(0.4 + 0.6 * d),
                            )
                            .into_any_element()
                    } else {
                        svg()
                            .path("icons/copy.svg")
                            .size(px(15.))
                            .text_color(t.text_muted)
                            .group_hover(SharedString::from(format!("copy-{index}")), move |s| {
                                s.text_color(t.text)
                            })
                            .into_any_element()
                    })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        this.copy_transcript(index, cx);
                    })),
            )
    }
}

fn stat_card(t: Theme, icon: &'static str, value: String, label: &'static str) -> gpui::Div {
    card(t)
        .flex_1()
        .gap(px(14.))
        .px(px(18.))
        .py(px(16.))
        .child(svg().path(icon).size(px(16.)).text_color(t.text_muted))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(2.))
                .child(
                    div()
                        .text_size(px(26.))
                        .line_height(px(30.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(t.text)
                        .child(value),
                )
                .child(div().text_size(px(12.)).text_color(t.text_muted).child(label)),
        )
}

fn group_thousands(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

impl MainWindow {
    fn render_stats(&self, t: Theme) -> impl IntoElement {
        let stats = compute_stats(&self.history);
        let cards = [
            stat_card(t, "icons/bolt.svg", stats.wpm.to_string(), "avg words per minute"),
            stat_card(t, "icons/text.svg", group_thousands(stats.total_words), "total words"),
            stat_card(t, "icons/flame.svg", stats.streak_days.to_string(), "day streak"),
        ];
        let first_show = self.nav_count == 0;
        div().flex().gap(px(12.)).pb(px(28.)).children(cards.into_iter().enumerate().map(
            move |(i, c)| {
                // First show only: cards rise in 40ms apart.
                if first_show {
                    div().flex_1().child(c.size_full()).map(|el| fade_in(el, ("stat-in", i), i as u64 * 40, true))
                } else {
                    div().flex_1().child(c.size_full()).into_any_element()
                }
            },
        ))
    }
}
