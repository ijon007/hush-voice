//! Spoken shortcuts. One trigger and its expansion per line in the app data
//! directory. Saying the trigger inserts the expansion in the transcript.

use std::time::Duration;

use gpui::{
    Animation, AnimationExt, Context, FontWeight, KeyDownEvent, MouseButton, SharedString, Timer,
    Window, div, ease_out_quint, prelude::*, px, rgba, svg,
};

use crate::main_window::pages::{page, page_header};
use crate::main_window::MainWindow;
use crate::models::Shortcut;
use crate::text_field::{self, Field};
use crate::theme::{RADIUS_PX, RADIUS_SM_PX, Theme, app_dir, theme};
use crate::widgets::{empty_state, fade_in};

pub(super) const TRIGGER_MAX: usize = 48;
pub(super) const EXPANSION_MAX: usize = 160;
const DIALOG_MS: u64 = 200;

#[derive(Clone, Copy, PartialEq, Eq)]
enum DialogField {
    Word,
    Shortcut,
}

pub(crate) fn load_shortcuts() -> Vec<Shortcut> {
    let Some(path) = app_dir().map(|dir| dir.join("shortcuts")) else {
        return Vec::new();
    };
    let Ok(raw) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    let mut shortcuts = Vec::new();
    for line in raw.lines() {
        let Some((trigger, expansion)) = line.split_once('\t') else { continue };
        upsert_shortcut(&mut shortcuts, trigger, expansion);
    }
    shortcuts
}

fn save_shortcuts(shortcuts: &[Shortcut]) {
    let Some(dir) = app_dir() else { return };
    let _ = std::fs::create_dir_all(&dir);
    let body = shortcuts
        .iter()
        .map(|s| format!("{}\t{}", s.trigger, s.expansion))
        .collect::<Vec<_>>()
        .join("\n");
    let _ = std::fs::write(dir.join("shortcuts"), body);
}

fn squash(raw: &str, max: usize) -> Option<String> {
    let text: String = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    let text: String = text.chars().take(max).collect();
    if text.is_empty() { None } else { Some(text) }
}

/// Insert or replace by trigger (case-insensitive). Keeps alphabetical order.
/// False when either side is empty.
pub(super) fn upsert_shortcut(shortcuts: &mut Vec<Shortcut>, trigger: &str, expansion: &str) -> bool {
    let (Some(trigger), Some(expansion)) = (squash(trigger, TRIGGER_MAX), squash(expansion, EXPANSION_MAX)) else {
        return false;
    };
    let key = trigger.to_lowercase();
    if let Some(existing) = shortcuts.iter_mut().find(|s| s.trigger.to_lowercase() == key) {
        existing.trigger = trigger.into();
        existing.expansion = expansion.into();
    } else {
        shortcuts.push(Shortcut::new(trigger, expansion));
    }
    shortcuts.sort_by(|a, b| a.trigger.to_lowercase().cmp(&b.trigger.to_lowercase()));
    true
}

fn matches_query(shortcut: &Shortcut, query: &str) -> bool {
    let q = query.trim().to_lowercase();
    q.is_empty()
        || shortcut.trigger.to_lowercase().contains(&q)
        || shortcut.expansion.to_lowercase().contains(&q)
}

impl MainWindow {
    pub(super) fn render_shortcuts(&self, window: &Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx);
        let focused = self.shortcut_focus.is_focused(window);
        let shown: Vec<(usize, &Shortcut)> = self
            .shortcuts
            .iter()
            .enumerate()
            .filter(|(_, shortcut)| matches_query(shortcut, &self.shortcut_query))
            .collect();

        let body = if shown.is_empty() {
            let (title, detail) = if self.shortcuts.is_empty() && self.shortcut_query.trim().is_empty() {
                ("No shortcuts yet", "Add a shortcut for a word you say")
            } else {
                ("Nothing matches", "Try a different word")
            };
            empty_state(t, title, detail).into_any_element()
        } else {
            div()
                .flex()
                .flex_wrap()
                .items_start()
                .gap(px(8.))
                .w_full()
                .children(shown.into_iter().map(|(index, shortcut)| {
                    let card = self.shortcut_card(t, index, shortcut, cx);
                    if self.shortcut_added.as_deref() == Some(shortcut.trigger.as_ref()) {
                        fade_in(card, ("shortcut-in", self.shortcut_nonce), 0, true)
                    } else {
                        card.into_any_element()
                    }
                }))
                .into_any_element()
        };

        page()
            .child(page_header(
                t,
                "Shortcuts",
                "Say a word, and the transcript writes something else",
            ))
            .child(self.shortcut_toolbar(t, focused, cx))
            .child(div().pt(px(16.)).child(body))
    }

    fn shortcut_toolbar(&self, t: Theme, focused: bool, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .gap(px(8.))
            .w_full()
            .child(self.shortcut_field(t, focused, cx))
            .child(self.add_shortcut_button(t, cx))
    }

    fn shortcut_field(&self, t: Theme, focused: bool, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("shortcut-query")
            .flex()
            .flex_1()
            .min_w(px(0.))
            .items_center()
            .gap(px(8.))
            .h(px(36.))
            .px(px(12.))
            .rounded(px(RADIUS_SM_PX))
            .bg(t.input_bg)
            .border_1()
            .border_color(if focused { t.primary } else { t.border })
            .cursor_text()
            .track_focus(&self.shortcut_focus)
            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| {
                window.focus(&this.shortcut_focus);
                cx.notify();
            }))
            .on_key_down(cx.listener(Self::on_shortcut_key))
            .child(
                svg()
                    .path("icons/search.svg")
                    .size(px(15.))
                    .flex_none()
                    .text_color(if focused { t.text_secondary } else { t.text_muted }),
            )
            .child(text_field::paint(&self.shortcut_query, focused, "Search shortcuts", t))
    }

    fn add_shortcut_button(&self, t: Theme, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("add-shortcut")
            .flex()
            .flex_none()
            .items_center()
            .gap(px(6.))
            .h(px(36.))
            .px(px(14.))
            .rounded(px(RADIUS_SM_PX))
            .bg(t.primary)
            .text_color(t.primary_text)
            .text_size(px(13.))
            .font_weight(FontWeight::MEDIUM)
            .hover(move |s| s.bg(t.primary_active))
            .active(|s| s.opacity(0.88))
            .cursor_pointer()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_click(cx.listener(|this, _, window, cx| this.open_shortcut_dialog(window, cx)))
            .child(svg().path("icons/plus.svg").size(px(14.)).text_color(t.primary_text))
            .child("Add shortcut")
    }

    fn shortcut_card(&self, t: Theme, index: usize, shortcut: &Shortcut, cx: &mut Context<Self>) -> gpui::Stateful<gpui::Div> {
        let group: SharedString = format!("shortcut-{index}").into();
        div()
            .id(("shortcut-card", index))
            .group(group.clone())
            .flex()
            .flex_none()
            .flex_col()
            .gap(px(4.))
            .w(px(248.))
            .px(px(14.))
            .pt(px(12.))
            .pb(px(14.))
            .rounded(px(RADIUS_PX))
            .bg(t.surface)
            .border_1()
            .border_color(t.border)
            .hover(move |s| s.bg(t.surface_hover).border_color(t.border_strong))
            .child(
                div()
                    .flex()
                    .items_start()
                    .justify_between()
                    .gap(px(8.))
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.))
                            .text_size(px(15.))
                            .line_height(px(20.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(t.text)
                            .whitespace_normal()
                            .child(shortcut.trigger.clone()),
                    )
                    .child(self.remove_shortcut_button(t, group, index, cx)),
            )
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap(px(6.))
                    .child(
                        svg()
                            .path("icons/shortcut-arrow.svg")
                            .w(px(22.))
                            .h(px(14.))
                            .flex_none()
                            .mt(px(2.))
                            .text_color(t.text_secondary),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.))
                            .text_size(px(13.))
                            .line_height(px(18.))
                            .text_color(t.text_secondary)
                            .whitespace_normal()
                            .child(shortcut.expansion.clone()),
                    ),
            )
    }

    fn remove_shortcut_button(
        &self,
        t: Theme,
        group: SharedString,
        index: usize,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .id(("shortcut-x", index))
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .size(px(22.))
            .rounded(px(6.))
            .opacity(0.)
            .group_hover(group, |s| s.opacity(1.))
            .hover(move |s| s.bg(t.surface_active))
            .active(|s| s.opacity(0.7))
            .cursor_pointer()
            .child(svg().path("icons/x.svg").size(px(12.)).text_color(t.text_muted))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_click(cx.listener(move |this, _, _, cx| {
                cx.stop_propagation();
                this.remove_shortcut(index, cx);
            }))
    }

    fn on_shortcut_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if !self.shortcut_focus.is_focused(window) {
            return;
        }
        if text_field::apply_key(&mut self.shortcut_query, &event.keystroke, cx) {
            cx.stop_propagation();
            cx.notify();
            return;
        }
        if event.keystroke.key == "escape" {
            if self.shortcut_query.is_empty() {
                window.blur();
            } else {
                self.shortcut_query.clear();
            }
            cx.stop_propagation();
            cx.notify();
        }
    }

    fn open_shortcut_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.shortcut_dialog && !self.dialog_closing {
            window.focus(&self.word_focus);
            return;
        }
        self.dialog_word.clear();
        self.dialog_shortcut.clear();
        self.dialog_closing = false;
        self.shortcut_dialog = true;
        self.dialog_nonce += 1;
        self.dialog_epoch += 1;
        window.focus(&self.word_focus);
        cx.notify();
    }

    fn dismiss_shortcut_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.shortcut_dialog || self.dialog_closing {
            return;
        }
        self.dialog_closing = true;
        self.dialog_nonce += 1;
        self.dialog_epoch += 1;
        let epoch = self.dialog_epoch;
        window.blur();
        cx.notify();
        cx.spawn(async move |this, cx| {
            Timer::after(Duration::from_millis(DIALOG_MS)).await;
            let _ = this.update(cx, |this, cx| {
                if this.dialog_epoch == epoch {
                    this.shortcut_dialog = false;
                    this.dialog_closing = false;
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn confirm_shortcut_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let word = self.dialog_word.as_str().to_owned();
        let shortcut = self.dialog_shortcut.as_str().to_owned();
        if !upsert_shortcut(&mut self.shortcuts, &word, &shortcut) {
            if squash(&self.dialog_word, TRIGGER_MAX).is_none() {
                window.focus(&self.word_focus);
            } else {
                window.focus(&self.dialog_shortcut_focus);
            }
            cx.notify();
            return;
        }
        save_shortcuts(&self.shortcuts);
        self.shortcut_added = squash(&word, TRIGGER_MAX);
        self.shortcut_nonce += 1;
        self.dismiss_shortcut_dialog(window, cx);
    }

    fn remove_shortcut(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(shortcut) = self.shortcuts.get(index) else { return };
        if self.shortcut_added.as_deref() == Some(shortcut.trigger.as_ref()) {
            self.shortcut_added = None;
        }
        self.shortcuts.remove(index);
        save_shortcuts(&self.shortcuts);
        cx.notify();
    }

    pub(super) fn shortcut_dialog(&self, t: Theme, window: &Window, cx: &mut Context<Self>) -> impl IntoElement {
        let closing = self.dialog_closing;
        let nonce = self.dialog_nonce;
        let panel = self.dialog_panel(t, window, cx).with_animation(
            ("shortcut-panel", nonce),
            Animation::new(Duration::from_millis(DIALOG_MS)).with_easing(ease_out_quint()),
            move |el, d| {
                let shown = if closing { 1. - d } else { d };
                el.mt(px(16. * (1. - shown)))
            },
        );
        div()
            .id("shortcut-scrim")
            .absolute()
            .inset_0()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgba(0x1312115c))
            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| {
                this.dismiss_shortcut_dialog(window, cx);
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if event.keystroke.key == "escape" && !event.keystroke.modifiers.modified() {
                    this.dismiss_shortcut_dialog(window, cx);
                }
            }))
            .child(panel)
            .with_animation(
                ("shortcut-scrim", nonce),
                Animation::new(Duration::from_millis(DIALOG_MS)).with_easing(ease_out_quint()),
                move |el, d| {
                    let shown = if closing { 1. - d } else { d };
                    el.opacity(shown)
                },
            )
    }

    fn dialog_panel(&self, t: Theme, window: &Window, cx: &mut Context<Self>) -> gpui::Stateful<gpui::Div> {
        let ready = squash(&self.dialog_word, TRIGGER_MAX).is_some()
            && squash(&self.dialog_shortcut, EXPANSION_MAX).is_some();
        div()
            .id("shortcut-dialog")
            .w(px(400.))
            .flex()
            .flex_col()
            .gap(px(14.))
            .px(px(20.))
            .pt(px(18.))
            .pb(px(16.))
            .rounded(px(16.))
            .bg(t.surface)
            .border_1()
            .border_color(t.border)
            .shadow_lg()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(
                div()
                    .text_size(px(17.))
                    .line_height(px(22.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(t.text)
                    .child("Add shortcut"),
            )
            .child(self.dialog_field(
                t,
                window,
                cx,
                "shortcut-word",
                DialogField::Word,
                "Word",
                "my email shortcut",
            ))
            .child(self.dialog_field(
                t,
                window,
                cx,
                "shortcut-writes",
                DialogField::Shortcut,
                "Shortcut",
                "you@example.com",
            ))
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap(px(8.))
                    .pt(px(2.))
                    .child(self.dialog_cancel(t, cx))
                    .child(self.dialog_confirm(t, ready, cx)),
            )
    }

    fn dialog_field(
        &self,
        t: Theme,
        window: &Window,
        cx: &mut Context<Self>,
        id: &'static str,
        field: DialogField,
        label: &'static str,
        placeholder: &'static str,
    ) -> impl IntoElement {
        let focus = match field {
            DialogField::Word => self.word_focus.clone(),
            DialogField::Shortcut => self.dialog_shortcut_focus.clone(),
        };
        let focused = focus.is_focused(window);
        let value = match field {
            DialogField::Word => &self.dialog_word,
            DialogField::Shortcut => &self.dialog_shortcut,
        };
        div()
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(
                div()
                    .pl(px(2.))
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(t.text_secondary)
                    .child(label),
            )
            .child(
                div()
                    .id(id)
                    .h(px(36.))
                    .flex()
                    .items_center()
                    .px(px(12.))
                    .rounded(px(RADIUS_SM_PX))
                    .bg(t.input_bg)
                    .border_1()
                    .border_color(if focused { t.primary } else { t.border })
                    .cursor_text()
                    .track_focus(&focus)
                    .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, window, cx| {
                        cx.stop_propagation();
                        match field {
                            DialogField::Word => window.focus(&this.word_focus),
                            DialogField::Shortcut => window.focus(&this.dialog_shortcut_focus),
                        }
                        cx.notify();
                    }))
                    .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                        this.on_dialog_key(field, event, window, cx);
                    }))
                    .child(text_field::paint(value, focused, placeholder, t)),
            )
    }

    fn dialog_cancel(&self, t: Theme, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("shortcut-cancel")
            .h(px(36.))
            .px(px(14.))
            .flex()
            .items_center()
            .rounded(px(RADIUS_SM_PX))
            .bg(t.track)
            .border_1()
            .border_color(t.border)
            .text_size(px(13.))
            .font_weight(FontWeight::MEDIUM)
            .text_color(t.text)
            .cursor_pointer()
            .hover(move |s| s.bg(t.surface_hover))
            .active(|s| s.opacity(0.85))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_click(cx.listener(|this, _, window, cx| this.dismiss_shortcut_dialog(window, cx)))
            .child("Cancel")
    }

    fn dialog_confirm(&self, t: Theme, ready: bool, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("shortcut-confirm")
            .h(px(36.))
            .px(px(14.))
            .flex()
            .items_center()
            .rounded(px(RADIUS_SM_PX))
            .bg(t.primary)
            .text_color(t.primary_text)
            .text_size(px(13.))
            .font_weight(FontWeight::MEDIUM)
            .when(!ready, |el| el.opacity(0.35))
            .cursor_pointer()
            .hover(move |s| s.bg(t.primary_active))
            .active(|s| s.opacity(0.88))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_click(cx.listener(|this, _, window, cx| this.confirm_shortcut_dialog(window, cx)))
            .child("Add shortcut")
    }

    fn on_dialog_key(
        &mut self,
        field: DialogField,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let focused = match field {
            DialogField::Word => self.word_focus.is_focused(window),
            DialogField::Shortcut => self.dialog_shortcut_focus.is_focused(window),
        };
        if !focused {
            return;
        }
        if text_field::apply_key(self.dialog_buf(field), &event.keystroke, cx) {
            cx.stop_propagation();
            cx.notify();
            return;
        }
        match event.keystroke.key.as_str() {
            "enter" => match field {
                DialogField::Word => window.focus(&self.dialog_shortcut_focus),
                DialogField::Shortcut => self.confirm_shortcut_dialog(window, cx),
            },
            "tab" => {
                let next = if event.keystroke.modifiers.shift { DialogField::Word } else { DialogField::Shortcut };
                if next != field {
                    match next {
                        DialogField::Word => window.focus(&self.word_focus),
                        DialogField::Shortcut => window.focus(&self.dialog_shortcut_focus),
                    }
                }
            }
            "escape" => self.dismiss_shortcut_dialog(window, cx),
            _ => return,
        }
        cx.stop_propagation();
        cx.notify();
    }

    fn dialog_buf(&mut self, field: DialogField) -> &mut Field {
        match field {
            DialogField::Word => &mut self.dialog_word,
            DialogField::Shortcut => &mut self.dialog_shortcut,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::apply_shortcuts;

    #[test]
    fn shortcut_rewrites_the_spoken_word() {
        let mut shortcuts = Vec::new();
        assert!(upsert_shortcut(&mut shortcuts, "  email ", "ada@example.com"));
        assert!(upsert_shortcut(&mut shortcuts, "Email", "ada@studio.com"));
        assert_eq!(shortcuts.len(), 1);
        assert_eq!(shortcuts[0].expansion.as_ref(), "ada@studio.com");
        assert_eq!(
            apply_shortcuts("Send it to email today.", &shortcuts),
            "Send it to ada@studio.com today."
        );
        assert_eq!(apply_shortcuts("emailing", &shortcuts), "emailing");
        assert!(!upsert_shortcut(&mut shortcuts, "email", "   "));
    }
}
