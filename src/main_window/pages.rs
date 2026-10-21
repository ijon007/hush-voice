use gpui::{Context, FontWeight, div, prelude::*, px};

use crate::main_window::MainWindow;
use crate::theme::{Theme, theme};
use crate::widgets::{empty_state, history_list, section_label};

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
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(section_label(t, "History"))
                    .child(history_list(t, self.history.clone())),
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
