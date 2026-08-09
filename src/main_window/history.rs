use gpui::{Context, FontWeight, div, prelude::*, px};

use crate::main_window::MainWindow;
use crate::theme::{
    BORDER, PRIMARY, PRIMARY_ACTIVE, PRIMARY_TEXT, RADIUS_PX, SURFACE, SURFACE_HOVER, TEXT,
    TEXT_MUTED, TEXT_SECONDARY,
};
use crate::widgets::{empty_history, history_row};

impl MainWindow {
    pub(super) fn render_history(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let rows: Vec<_> = self.filtered_history().cloned().collect();

        div()
            .flex()
            .flex_col()
            .gap(px(12.))
            .w_full()
            .child(self.render_history_toolbar(cx))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .children(if rows.is_empty() {
                        vec![empty_history().into_any_element()]
                    } else {
                        rows.into_iter()
                            .map(|row| history_row(row).into_any_element())
                            .collect()
                    }),
            )
    }

    fn render_history_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap(px(10.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .px(px(12.))
                    .h(px(36.))
                    .rounded(px(RADIUS_PX))
                    .bg(SURFACE)
                    .border_1()
                    .border_color(BORDER)
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(TEXT_MUTED)
                            .child("Search transcripts…"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap(px(6.))
                    .child(self.filter_chip("All", None, cx))
                    .child(self.filter_chip("Whisper Local", Some("Whisper Local"), cx))
                    .child(self.filter_chip("Cloud STT", Some("Cloud STT"), cx)),
            )
    }

    fn filter_chip(
        &self,
        label: &'static str,
        filter: Option<&'static str>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selected = self.history_filter == filter;
        div()
            .id(label)
            .px(px(10.))
            .py(px(5.))
            .rounded(px(8.))
            .text_size(px(12.))
            .font_weight(FontWeight::MEDIUM)
            .text_color(if selected { PRIMARY_TEXT } else { TEXT_SECONDARY })
            .bg(if selected { PRIMARY } else { SURFACE })
            .border_1()
            .border_color(if selected { PRIMARY } else { BORDER })
            .hover(|s| {
                if selected {
                    s.bg(PRIMARY_ACTIVE)
                } else {
                    s.bg(SURFACE_HOVER).text_color(TEXT)
                }
            })
            .active(|s| s.opacity(0.9))
            .cursor_pointer()
            .child(label)
            .on_click(cx.listener(move |this, _, _, cx| this.set_history_filter(filter, cx)))
    }
}
