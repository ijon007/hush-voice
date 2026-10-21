use gpui::{Context, FontWeight, div, prelude::*, px, svg};

use crate::main_window::MainWindow;
use crate::models::ModelOption;
use crate::theme::{RADIUS_PX, RADIUS_SM_PX, Theme, ThemeMode, set_theme_mode, theme, theme_mode};
use crate::widgets::{card, primary_button, section_label, selection_dot};

fn section(t: Theme, label: &'static str, body: impl IntoElement) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap(px(8.))
        .child(section_label(t, label))
        .child(body)
}

fn row_text(t: Theme, title: &'static str, detail: &'static str) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap(px(2.))
        .child(
            div()
                .text_size(px(14.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(t.text)
                .child(title),
        )
        .child(div().text_size(px(12.)).text_color(t.text_muted).child(detail))
}

impl MainWindow {
    pub(super) fn render_settings(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx);
        div()
            .flex()
            .flex_col()
            .gap(px(24.))
            .w_full()
            .child(section(t, "Appearance", self.render_appearance(t, cx)))
            .child(section(t, "Model", self.render_model_list(t, cx)))
            .child(section(t, "Hotkey", self.render_hotkey(t)))
            .child(
                div()
                    .flex()
                    .justify_end()
                    .child(primary_button(t, "Save changes", "save-settings")),
            )
    }

    fn render_appearance(&self, t: Theme, cx: &mut Context<Self>) -> impl IntoElement {
        let current = theme_mode(cx);
        card(t)
            .flex_col()
            .gap(px(16.))
            .px(px(16.))
            .py(px(16.))
            .child(row_text(t, "Theme", "Choose light or dark"))
            .child(
                div()
                    .flex()
                    .justify_center()
                    .gap(px(24.))
                    .child(self.theme_preview(t, current, "Light", ThemeMode::Light, Theme::LIGHT, cx))
                    .child(self.theme_preview(t, current, "Dark", ThemeMode::Dark, Theme::DARK, cx)),
            )
    }

    fn theme_preview(
        &self,
        t: Theme,
        current: ThemeMode,
        label: &'static str,
        mode: ThemeMode,
        preview: Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selected = current == mode;
        div()
            .id(label)
            .flex()
            .flex_col()
            .items_center()
            .gap(px(10.))
            .cursor_pointer()
            .active(|s| s.opacity(0.92))
            .child(
                div()
                    .p(px(3.))
                    .rounded(px(12.))
                    .border_2()
                    .border_color(if selected { t.primary } else { t.surface })
                    .child(home_miniature(preview)),
            )
            .child(
                div()
                    .text_size(px(13.))
                    .font_weight(if selected { FontWeight::SEMIBOLD } else { FontWeight::NORMAL })
                    .text_color(if selected { t.text } else { t.text_secondary })
                    .child(label),
            )
            .on_click(cx.listener(move |_, _, _, cx| {
                set_theme_mode(mode, cx);
                cx.notify();
            }))
    }

    fn render_model_list(&self, t: Theme, cx: &mut Context<Self>) -> impl IntoElement {
        let last = self.models.len().saturating_sub(1);
        let mut rows = Vec::new();
        for (index, model) in self.models.iter().enumerate() {
            rows.push(self.model_row(t, index, model, index < last, cx).into_any_element());
        }
        card(t).children(rows)
    }

    fn model_row(
        &self,
        t: Theme,
        index: usize,
        model: &ModelOption,
        show_divider: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selected = self.active_model == index;
        div()
            .id(model.id)
            .flex()
            .items_center()
            .justify_between()
            .px(px(16.))
            .py(px(14.))
            .when(index == 0, |el| el.rounded_t(px(RADIUS_PX - 1.)))
            .when(!show_divider, |el| el.rounded_b(px(RADIUS_PX - 1.)))
            .when(show_divider, |el| el.border_b_1().border_color(t.border))
            .when(selected, |el| el.bg(t.surface_selected))
            .when(!selected, |el| el.hover(move |s| s.bg(t.surface_hover)))
            .active(move |s| s.bg(t.surface_active))
            .cursor_pointer()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .child(
                        div()
                            .text_size(px(14.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(t.text)
                            .child(model.name.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(t.text_muted)
                            .child(model.detail.clone()),
                    ),
            )
            .child(selection_dot(t, selected, self.dot_nonce))
            .on_click(cx.listener(move |this, _, _, cx| this.select_model(index, cx)))
    }

    fn render_hotkey(&self, t: Theme) -> impl IntoElement {
        card(t)
            .flex_row()
            .items_center()
            .justify_between()
            .px(px(16.))
            .py(px(14.))
            .child(row_text(t, "Push to talk", "Hold to show overlay, release to hide"))
            .child(
                div()
                    .px(px(10.))
                    .py(px(5.))
                    .rounded(px(RADIUS_SM_PX))
                    .bg(t.track)
                    .border_1()
                    .border_color(t.border_strong)
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(t.text)
                    .child(self.hotkey.clone()),
            )
    }
}

/// Tiny Home tab, painted in a fixed palette so Light and Dark can sit side by side.
fn home_miniature(t: Theme) -> impl IntoElement {
    div()
        .w(px(228.))
        .h(px(148.))
        .rounded(px(10.))
        .overflow_hidden()
        .bg(t.bg)
        .border_1()
        .border_color(t.border_strong)
        .shadow_sm()
        .flex()
        .flex_col()
        .child(
            div()
                .h(px(16.))
                .flex_none()
                .flex()
                .items_center()
                .px(px(8.))
                .child(
                    div()
                        .text_size(px(7.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(t.text)
                        .child("Hush"),
                ),
        )
        .child(
            div()
                .flex()
                .flex_1()
                .min_h(px(0.))
                .child(mini_sidebar(t))
                .child(mini_home_panel(t)),
        )
}

fn mini_sidebar(t: Theme) -> impl IntoElement {
    div()
        .w(px(52.))
        .h_full()
        .flex_none()
        .flex()
        .flex_col()
        .justify_between()
        .px(px(4.))
        .py(px(4.))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(2.))
                .child(mini_nav(t, "icons/home.svg", true))
                .child(mini_nav(t, "icons/book.svg", false))
                .child(mini_nav(t, "icons/keyboard.svg", false)),
        )
        .child(mini_nav(t, "icons/settings.svg", false))
}

fn mini_nav(t: Theme, icon: &'static str, selected: bool) -> impl IntoElement {
    let fg = if selected { t.text } else { t.text_secondary };
    div()
        .h(px(14.))
        .px(px(4.))
        .rounded(px(3.))
        .flex()
        .items_center()
        .when(selected, |el| el.bg(t.nav_selected))
        .child(svg().path(icon).size(px(8.)).text_color(fg))
}

fn mini_home_panel(t: Theme) -> impl IntoElement {
    div()
        .flex_1()
        .h_full()
        .min_w(px(0.))
        .pr(px(4.))
        .pb(px(4.))
        .child(
            div()
                .size_full()
                .rounded(px(6.))
                .bg(t.panel)
                .overflow_hidden()
                .px(px(10.))
                .pt(px(8.))
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(
                            div()
                                .text_size(px(9.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(t.text)
                                .child("Welcome back!"),
                        )
                        .child(
                            div()
                                .text_size(px(6.))
                                .text_color(t.text_muted)
                                .child("Hold Ctrl + Space to dictate"),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .gap(px(4.))
                        .child(mini_stat(t, "142"))
                        .child(mini_stat(t, "3,184"))
                        .child(mini_stat(t, "4")),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(3.))
                        .child(
                            div()
                                .text_size(px(6.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(t.text_secondary)
                                .child("History"),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .rounded(px(6.))
                                .bg(t.surface)
                                .border_1()
                                .border_color(t.border)
                                .overflow_hidden()
                                .child(mini_history_row(t, "Schedule a design review\u{2026}", true))
                                .child(mini_history_row(t, "Remind me to send the notes\u{2026}", false)),
                        ),
                ),
        )
}

fn mini_stat(t: Theme, value: &'static str) -> impl IntoElement {
    div()
        .flex_1()
        .rounded(px(6.))
        .bg(t.surface)
        .border_1()
        .border_color(t.border)
        .px(px(6.))
        .py(px(5.))
        .child(
            div()
                .text_size(px(9.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(t.text)
                .child(value),
        )
}

fn mini_history_row(t: Theme, text: &'static str, divider: bool) -> impl IntoElement {
    div()
        .px(px(6.))
        .py(px(5.))
        .when(divider, |el| el.border_b_1().border_color(t.border))
        .child(div().text_size(px(6.)).text_color(t.text).child(text))
}
