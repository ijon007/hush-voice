use gpui::{Context, FontWeight, div, prelude::*, px};

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
            .flex_row()
            .items_center()
            .justify_between()
            .px(px(16.))
            .py(px(14.))
            .child(row_text(t, "Theme", "Choose light, dark, or follow Windows"))
            .child(
                div()
                    .flex()
                    .p(px(3.))
                    .gap(px(2.))
                    .rounded(px(RADIUS_PX - 2.))
                    .bg(t.track)
                    .child(self.theme_option(t, current, "Light", ThemeMode::Light, cx))
                    .child(self.theme_option(t, current, "Dark", ThemeMode::Dark, cx))
                    .child(self.theme_option(t, current, "System", ThemeMode::System, cx)),
            )
    }

    fn theme_option(
        &self,
        t: Theme,
        current: ThemeMode,
        label: &'static str,
        mode: ThemeMode,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selected = current == mode;
        div()
            .id(label)
            .px(px(14.))
            .py(px(5.))
            .rounded(px(RADIUS_SM_PX - 1.))
            .text_size(px(13.))
            .font_weight(if selected { FontWeight::MEDIUM } else { FontWeight::NORMAL })
            .text_color(if selected { t.text } else { t.text_secondary })
            .when(selected, |el| el.bg(t.surface).shadow_sm())
            .hover(move |s| if selected { s } else { s.bg(t.surface_hover).text_color(t.text) })
            .active(|s| s.opacity(0.85))
            .cursor_pointer()
            .child(label)
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
            .when(show_divider, |el| el.border_b_1().border_color(t.border))
            .when(selected, |el| el.bg(t.surface_selected))
            .hover(move |s| s.bg(t.surface_hover))
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
            .child(selection_dot(t, selected))
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
