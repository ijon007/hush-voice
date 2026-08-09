use gpui::{
    App, FontWeight, MouseButton, SharedString, Window, div, prelude::*, px, rgb,
};

use crate::models::TranscriptRow;
use crate::theme::{
    BORDER, BORDER_STRONG, PRIMARY, PRIMARY_ACTIVE, PRIMARY_TEXT, RADIUS_PX, RADIUS_SM_PX, SURFACE,
    SURFACE_HOVER, TEXT, TEXT_MUTED, TEXT_SECONDARY, TRACK,
};

pub(crate) fn window_btn(
    id: &'static str,
    label: &'static str,
    on_click: impl Fn(&mut Window, &mut App) + 'static,
) -> gpui::AnyElement {
    let is_close = id == "win-close";
    div()
        .id(id)
        .group(id)
        .w(px(46.))
        .h(px(36.))
        .flex()
        .items_center()
        .justify_center()
        .hover(move |s| {
            if is_close {
                s.bg(rgb(0xc42b1c))
            } else {
                s.bg(TRACK)
            }
        })
        .active(move |s| {
            if is_close {
                s.bg(rgb(0xa52318))
            } else {
                s.bg(BORDER)
            }
        })
        .cursor_pointer()
        .child(
            div()
                .text_size(px(12.))
                .text_color(TEXT_SECONDARY)
                .group_hover(id, move |s| {
                    if is_close {
                        s.text_color(rgb(0xffffff))
                    } else {
                        s.text_color(TEXT)
                    }
                })
                .child(label),
        )
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_click(move |_, window, cx| on_click(window, cx))
        .into_any_element()
}

pub(crate) fn section_label(text: &'static str) -> impl IntoElement {
    div()
        .pl(px(4.))
        .pb(px(2.))
        .text_size(px(13.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(TEXT_SECONDARY)
        .child(text)
}

pub(crate) fn selection_dot(selected: bool) -> impl IntoElement {
    div()
        .size(px(18.))
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
                    .child(div().size(px(6.)).rounded_full().bg(PRIMARY_TEXT)),
            )
        })
}

pub(crate) fn primary_button(label: &'static str, id: &'static str) -> impl IntoElement {
    div()
        .id(id)
        .px(px(16.))
        .py(px(9.))
        .rounded(px(RADIUS_SM_PX))
        .bg(PRIMARY)
        .text_color(PRIMARY_TEXT)
        .text_size(px(13.))
        .font_weight(FontWeight::MEDIUM)
        .hover(|s| s.bg(PRIMARY_ACTIVE))
        .active(|s| s.opacity(0.92))
        .cursor_pointer()
        .child(label)
}

/// Grouped transcript list — one surface, hairline dividers. No cards, no pills.
pub(crate) fn history_list(rows: Vec<TranscriptRow>) -> impl IntoElement {
    if rows.is_empty() {
        return empty_history().into_any_element();
    }

    let last = rows.len().saturating_sub(1);
    div()
        .flex()
        .flex_col()
        .w_full()
        .rounded(px(RADIUS_PX))
        .bg(SURFACE)
        .border_1()
        .border_color(BORDER)
        .overflow_hidden()
        .children(
            rows.into_iter()
                .enumerate()
                .map(|(i, row)| history_row(row, i < last).into_any_element()),
        )
        .into_any_element()
}

fn history_row(row: TranscriptRow, show_divider: bool) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap(px(4.))
        .px(px(16.))
        .py(px(14.))
        .hover(|s| s.bg(SURFACE_HOVER))
        .when(show_divider, |el| el.border_b_1().border_color(BORDER))
        .child(
            div()
                .text_size(px(14.))
                .text_color(TEXT)
                .line_height(px(20.))
                .child(row.text),
        )
        .child(meta_line(row.model, row.duration))
}

fn meta_line(model: SharedString, duration: SharedString) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap(px(6.))
        .text_size(px(12.))
        .text_color(TEXT_MUTED)
        .child(model)
        .child(div().text_color(BORDER_STRONG).child("·"))
        .child(duration)
}

pub(crate) fn empty_history() -> impl IntoElement {
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
                .child("No transcripts yet"),
        )
        .child(
            div()
                .text_size(px(13.))
                .text_color(TEXT_MUTED)
                .child("Hold Ctrl + Space to capture the first one"),
        )
}
