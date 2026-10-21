use gpui::{App, FontWeight, MouseButton, Window, div, prelude::*, px, rgb};

use crate::theme::{RADIUS_PX, RADIUS_SM_PX, Theme};

pub(crate) fn window_btn(
    t: Theme,
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
        .hover(move |s| if is_close { s.bg(t.close_hover) } else { s.bg(t.nav_hover) })
        .active(move |s| if is_close { s.bg(t.close_active) } else { s.bg(t.nav_selected) })
        .cursor_pointer()
        .child(
            div()
                .text_size(px(12.))
                .text_color(t.text_secondary)
                .group_hover(id, move |s| {
                    if is_close {
                        s.text_color(rgb(0xffffff))
                    } else {
                        s.text_color(t.text)
                    }
                })
                .child(label),
        )
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_click(move |_, window, cx| on_click(window, cx))
        .into_any_element()
}

pub(crate) fn section_label(t: Theme, text: &'static str) -> impl IntoElement {
    div()
        .pl(px(4.))
        .pb(px(2.))
        .text_size(px(13.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(t.text_secondary)
        .child(text)
}

pub(crate) fn selection_dot(t: Theme, selected: bool) -> impl IntoElement {
    div()
        .size(px(18.))
        .rounded_full()
        .border_1()
        .border_color(if selected { t.primary } else { t.border_strong })
        .when(selected, |el| {
            el.bg(t.primary).child(
                div()
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(div().size(px(6.)).rounded_full().bg(t.primary_text)),
            )
        })
}

pub(crate) fn primary_button(t: Theme, label: &'static str, id: &'static str) -> impl IntoElement {
    div()
        .id(id)
        .px(px(16.))
        .py(px(9.))
        .rounded(px(RADIUS_SM_PX))
        .bg(t.primary)
        .text_color(t.primary_text)
        .text_size(px(13.))
        .font_weight(FontWeight::MEDIUM)
        .hover(move |s| s.bg(t.primary_active))
        .active(|s| s.opacity(0.92))
        .cursor_pointer()
        .child(label)
}

/// Grouped card surface used by lists and placeholders.
pub(crate) fn card(t: Theme) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .w_full()
        .rounded(px(RADIUS_PX))
        .bg(t.surface)
        .border_1()
        .border_color(t.border)
        .overflow_hidden()
}

pub(crate) fn empty_state(t: Theme, title: &'static str, body: &'static str) -> impl IntoElement {
    card(t)
        .items_center()
        .justify_center()
        .gap(px(6.))
        .py(px(72.))
        .child(
            div()
                .text_size(px(15.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(t.text)
                .child(title),
        )
        .child(div().text_size(px(13.)).text_color(t.text_muted).child(body))
}
