use gpui::{Context, FontWeight, SharedString, div, prelude::*, px};

use crate::main_window::MainWindow;
use crate::theme::{BORDER, RADIUS_PX, SURFACE, TEXT, TEXT_MUTED};
use crate::widgets::section_label;

pub(super) fn page_header(title: &'static str, subtitle: &'static str) -> impl IntoElement {
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
                .text_color(TEXT)
                .child(title),
        )
        .child(div().text_size(px(13.)).text_color(TEXT_MUTED).child(subtitle))
}

fn placeholder(title: &'static str, body: &'static str) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(6.))
        .py(px(72.))
        .rounded(px(RADIUS_PX))
        .bg(SURFACE)
        .border_1()
        .border_color(BORDER)
        .child(
            div()
                .text_size(px(15.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(TEXT)
                .child(SharedString::from(title)),
        )
        .child(div().text_size(px(13.)).text_color(TEXT_MUTED).child(body))
}

impl MainWindow {
    pub(super) fn render_home(&self) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .w_full()
            .child(page_header("Welcome back!", "Hold Ctrl + Space to dictate"))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(section_label("History"))
                    .child(self.render_history()),
            )
    }

    pub(super) fn render_settings_page(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .w_full()
            .child(page_header("Settings", "Model, hotkey and appearance"))
            .child(self.render_settings(cx))
    }

    pub(super) fn render_dictionary(&self) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .w_full()
            .child(page_header("Dictionary", "Custom words and spellings"))
            .child(placeholder("Nothing here yet", "Your custom vocabulary will live here"))
    }

    pub(super) fn render_shortcuts(&self) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .w_full()
            .child(page_header("Shortcuts", "Keyboard shortcuts and snippets"))
            .child(placeholder("Nothing here yet", "Your shortcuts will live here"))
    }
}
