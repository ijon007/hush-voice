use std::time::Duration;

use gpui::{
    Animation, AnimationExt, App, ElementId, FontWeight, MouseButton, Window, div, ease_out_quint,
    prelude::*, px, rgb,
};

use crate::theme::{RADIUS_PX, Theme};

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

pub(crate) fn selection_dot(t: Theme, selected: bool, anim_key: usize) -> impl IntoElement {
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
                    .child(
                        // Inner dot scales in when a row becomes selected.
                        div().size(px(6.)).rounded_full().bg(t.primary_text).with_animation(
                            ("radio-dot", anim_key),
                            Animation::new(Duration::from_millis(160)).with_easing(ease_out_quint()),
                            |el, d| el.size(px(2. + 4. * d)),
                        ),
                    ),
            )
        })
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

/// Staggered entrance: fades (and optionally lifts) an element in after
/// `delay_ms`, over 220ms with an ease-out cubic.
pub(crate) fn fade_in<E: IntoElement + Styled + 'static>(
    el: E,
    id: impl Into<ElementId>,
    delay_ms: u64,
    lift: bool,
) -> gpui::AnyElement {
    const RUN_MS: f32 = 220.;
    let delay = delay_ms as f32;
    let total = RUN_MS + delay;
    el.with_animation(
        id,
        Animation::new(Duration::from_millis(total as u64)).with_easing(move |t| {
            let x = ((t * total - delay) / RUN_MS).clamp(0., 1.);
            1. - (1. - x).powi(3)
        }),
        move |el, d| {
            let el = el.opacity(d);
            if lift { el.mt(px(6. * (1. - d))) } else { el }
        },
    )
    .into_any_element()
}

/// cubic-bezier(0.23, 1, 0.32, 1): a strong ease-out for UI motion.
pub(crate) fn ease_strong(x: f32) -> f32 {
    let bez = |a: f32, b: f32, t: f32| {
        let u = 1. - t;
        3. * u * u * t * a + 3. * u * t * t * b + t * t * t
    };
    let x = x.clamp(0., 1.);
    let (mut lo, mut hi) = (0f32, 1f32);
    for _ in 0..20 {
        let mid = (lo + hi) / 2.;
        if bez(0.23, 0.32, mid) < x { lo = mid } else { hi = mid }
    }
    bez(1., 1., (lo + hi) / 2.)
}
