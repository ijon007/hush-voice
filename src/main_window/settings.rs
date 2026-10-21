use gpui::{Context, FontWeight, div, prelude::*, px};

use crate::main_window::MainWindow;
use crate::models::ModelOption;
use crate::theme::{
    BORDER, BORDER_STRONG, RADIUS_PX, RADIUS_SM_PX, SURFACE, SURFACE_ACTIVE, SURFACE_HOVER, SURFACE_SELECTED, TEXT,
    TEXT_MUTED, TRACK,
};
use crate::widgets::{primary_button, section_label, selection_dot};

impl MainWindow {
    pub(super) fn render_settings(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap(px(24.))
            .w_full()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(section_label("Model"))
                    .child(self.render_model_list(cx)),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(section_label("Hotkey"))
                    .child(self.render_hotkey()),
            )
            .child(
                div()
                    .flex()
                    .justify_end()
                    .child(primary_button("Save changes", "save-settings")),
            )
    }

    fn render_model_list(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let last = self.models.len().saturating_sub(1);
        let mut rows = Vec::new();
        for (index, model) in self.models.iter().enumerate() {
            rows.push(
                self.model_row(index, model, index < last, cx)
                    .into_any_element(),
            );
        }
        div()
            .flex()
            .flex_col()
            .rounded(px(RADIUS_PX))
            .bg(SURFACE)
            .border_1()
            .border_color(BORDER)
            .overflow_hidden()
            .children(rows)
    }

    fn model_row(
        &self,
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
            .when(show_divider, |el| el.border_b_1().border_color(BORDER))
            .when(selected, |el| el.bg(SURFACE_SELECTED))
            .hover(|s| s.bg(SURFACE_HOVER))
            .active(|s| s.bg(SURFACE_ACTIVE).opacity(0.95))
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
                            .text_color(TEXT)
                            .child(model.name.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(TEXT_MUTED)
                            .child(model.detail.clone()),
                    ),
            )
            .child(selection_dot(selected))
            .on_click(cx.listener(move |this, _, _, cx| this.select_model(index, cx)))
    }

    fn render_hotkey(&self) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .justify_between()
            .px(px(16.))
            .py(px(14.))
            .rounded(px(RADIUS_PX))
            .bg(SURFACE)
            .border_1()
            .border_color(BORDER)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .child(
                        div()
                            .text_size(px(14.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(TEXT)
                            .child("Push to talk"),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(TEXT_MUTED)
                            .child("Hold to show overlay, release to hide"),
                    ),
            )
            .child(
                div()
                    .px(px(10.))
                    .py(px(5.))
                    .rounded(px(RADIUS_SM_PX))
                    .bg(TRACK)
                    .border_1()
                    .border_color(BORDER_STRONG)
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(TEXT)
                    .child(self.hotkey.clone()),
            )
    }
}
