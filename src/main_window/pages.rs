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

pub(super) fn page() -> gpui::Div {
    div().flex().flex_col().w_full()
}

impl MainWindow {
    pub(super) fn render_home(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx);
        let p = crate::prefs::prefs(cx).clone();
        let ready = crate::engine::resolve(&p.model, p.custom_model.as_deref()).is_some();
        page()
            .child(page_header(t, "Welcome back!", "Hold Ctrl + Space to dictate"))
            .when(!ready, |el| el.child(self.render_setup(t, cx)))
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
            .child(page_header(t, "Settings", "How Hush starts, listens and looks"))
            .child(self.render_settings(cx))
    }

}

impl MainWindow {
    /// First run: no speech model on disk yet.
    fn render_setup(&self, t: Theme, cx: &mut Context<Self>) -> impl IntoElement {
        div().pb(px(20.)).child(
            card(t)
                .flex_row()
                .items_center()
                .gap(px(24.))
                .px(px(20.))
                .py(px(18.))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .flex_1()
                        .gap(px(4.))
                        .child(
                            div()
                                .text_size(px(15.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(t.text)
                                .child("Download a model to start"),
                        )
                        .child(
                            div()
                                .text_size(px(13.))
                                .text_color(t.text_muted)
                                .child("Hush turns speech into text right on this PC. Balanced is a good first pick."),
                        ),
                )
                .child(
                    div()
                        .id("open-models")
                        .flex_none()
                        .px(px(14.))
                        .h(px(34.))
                        .flex()
                        .items_center()
                        .rounded(px(crate::theme::RADIUS_SM_PX))
                        .bg(t.primary)
                        .text_color(t.primary_text)
                        .text_size(px(13.))
                        .font_weight(FontWeight::MEDIUM)
                        .hover(move |s| s.bg(t.primary_active))
                        .active(|s| s.opacity(0.88))
                        .cursor_pointer()
                        .child("Choose a model")
                        .on_click(cx.listener(|this, _, _, cx| this.open_model_settings(cx))),
                ),
        )
    }

    /// Grouped transcript list: one surface, hairline dividers, copy on hover.
    fn render_history(&self, t: Theme, cx: &mut Context<Self>) -> gpui::AnyElement {
        if self.history.is_empty() {
            return empty_state(t, "No dictations yet", "Hold Ctrl + Space, speak, and let go")
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

/// One stat column: bare icon + label on top, big number (+ muted unit) below.
/// Identical structure in every column so the baselines line up.
fn stat_cell(
    t: Theme,
    icon: &'static str,
    value: String,
    unit: Option<&'static str>,
    label: &'static str,
    divider: bool,
) -> impl IntoElement {
    div()
        .flex_1()
        .min_w(px(0.))
        .flex()
        .flex_col()
        .gap(px(10.))
        .px(px(20.))
        .pt(px(16.))
        .pb(px(18.))
        .when(divider, |el| el.border_l_1().border_color(t.border))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.))
                .h(px(16.))
                .child(svg().path(icon).size(px(14.)).flex_none().text_color(t.text_muted))
                .child(
                    div()
                        .text_size(px(12.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(t.text_muted)
                        .truncate()
                        .child(label),
                ),
        )
        .child(
            div()
                .flex()
                .items_baseline()
                .gap(px(4.))
                .h(px(34.))
                .child(
                    div()
                        .text_size(px(30.))
                        .line_height(px(34.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(t.text)
                        .child(value),
                )
                .when_some(unit, |el, unit| {
                    el.child(
                        div()
                            .text_size(px(14.))
                            .text_color(t.text_muted)
                            .child(unit),
                    )
                }),
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
        let streak_unit = if stats.streak_days == 1 { "day" } else { "days" };
        let grouped = card(t)
            .flex_row()
            .child(stat_cell(t, "icons/bolt.svg", stats.wpm.to_string(), Some("wpm"), "Average speed", false))
            .child(stat_cell(t, "icons/text.svg", group_thousands(stats.total_words), None, "Words dictated", true))
            .child(stat_cell(
                t,
                "icons/flame.svg",
                stats.streak_days.to_string(),
                Some(streak_unit),
                "Current streak",
                true,
            ));
        let wrap = div().pb(px(28.));
        // First show only: the stats card rises in once.
        if self.nav_count == 0 {
            wrap.child(fade_in(grouped, "stats-in", 0, true)).into_any_element()
        } else {
            wrap.child(grouped).into_any_element()
        }
    }
}
