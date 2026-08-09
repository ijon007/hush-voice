use gpui::{
    App, FontWeight, MouseButton, SharedString, Window, div, prelude::*, px, rgb,
};

use crate::models::TranscriptRow;
use crate::theme::{
    BG, BORDER, BORDER_STRONG, PRIMARY, PRIMARY_ACTIVE, PRIMARY_TEXT, RADIUS_PX, SURFACE,
    SURFACE_HOVER, TEXT, TEXT_MUTED, TEXT_SECONDARY,
};

pub(crate) fn window_btn(
    id: &'static str,
    label: &'static str,
    on_click: impl Fn(&mut Window, &mut App) + 'static,
) -> gpui::AnyElement {
    let is_close = id == "win-close";
    div()
        .id(id)
        .w(px(46.))
        .h(px(36.))
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(12.))
        .text_color(TEXT_SECONDARY)
        .hover(move |s| {
            if is_close {
                s.bg(rgb(0xc42b1c)).text_color(TEXT)
            } else {
                s.bg(SURFACE_HOVER).text_color(TEXT)
            }
        })
        .active(|s| s.opacity(0.9))
        .cursor_pointer()
        .child(label)
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_click(move |_, window, cx| on_click(window, cx))
        .into_any_element()
}

pub(crate) fn section_label(text: &'static str) -> impl IntoElement {
    div()
        .text_size(px(11.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(TEXT_MUTED)
        .child(text.to_uppercase())
}

pub(crate) fn selection_dot(selected: bool) -> impl IntoElement {
    div()
        .size(px(14.))
        .rounded_full()
        .border_1()
        .border_color(if selected { PRIMARY } else { BORDER_STRONG })
        .when(selected, |el| {
            el.bg(PRIMARY).child(
                div()
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(div().size(px(5.)).rounded_full().bg(PRIMARY_TEXT)),
            )
        })
}

pub(crate) fn primary_button(label: &'static str, id: &'static str) -> impl IntoElement {
    div()
        .id(id)
        .px(px(14.))
        .py(px(8.))
        .rounded(px(RADIUS_PX))
        .bg(PRIMARY)
        .text_color(PRIMARY_TEXT)
        .text_size(px(13.))
        .font_weight(FontWeight::MEDIUM)
        .hover(|s| s.bg(PRIMARY_ACTIVE))
        .active(|s| s.opacity(0.92))
        .cursor_pointer()
        .child(label)
}

pub(crate) fn history_row(row: TranscriptRow) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap(px(8.))
        .px(px(14.))
        .py(px(12.))
        .rounded(px(RADIUS_PX))
        .bg(SURFACE)
        .border_1()
        .border_color(BORDER)
        .hover(|s| s.bg(SURFACE_HOVER).border_color(BORDER_STRONG))
        .child(
            div()
                .text_size(px(13.))
                .text_color(TEXT)
                .line_height(px(20.))
                .child(row.text),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(10.))
                .child(meta_pill(row.model))
                .child(meta_text(row.duration)),
        )
}

fn meta_pill(text: SharedString) -> impl IntoElement {
    div()
        .px(px(7.))
        .py(px(2.))
        .rounded(px(6.))
        .bg(BG)
        .border_1()
        .border_color(BORDER)
        .text_size(px(11.))
        .text_color(TEXT_SECONDARY)
        .child(text)
}

fn meta_text(text: SharedString) -> impl IntoElement {
    div()
        .text_size(px(11.))
        .text_color(TEXT_MUTED)
        .child(text)
}

pub(crate) fn empty_history() -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(6.))
        .py(px(56.))
        .child(
            div()
                .text_size(px(14.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(TEXT_SECONDARY)
                .child("No transcripts yet"),
        )
        .child(
            div()
                .text_size(px(12.))
                .text_color(TEXT_MUTED)
                .child("Hold Ctrl + Space to capture the first one"),
        )
}
