//! Custom vocabulary. One word per line in the app data directory, so a speech
//! model can prefer these spellings when a token is uncertain.

use std::time::Duration;

use gpui::{
    Animation, AnimationExt, Context, FontWeight, KeyDownEvent, MouseButton, SharedString, Timer,
    Window, div, ease_out_quint, prelude::*, px, rgba, svg,
};

use crate::main_window::pages::{page, page_header};
use crate::main_window::MainWindow;
use crate::theme::{RADIUS_SM_PX, Theme, app_dir, theme};
use crate::widgets::{empty_state, fade_in};

const MAX_LEN: usize = 48;
const DIALOG_MS: u64 = 200;

pub(crate) fn load_words() -> Vec<String> {
    let Some(path) = app_dir().map(|dir| dir.join("dictionary")) else {
        return Vec::new();
    };
    let Ok(raw) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    let mut words = Vec::new();
    for line in raw.lines() {
        insert_word(&mut words, line);
    }
    words
}

fn save_words(words: &[String]) {
    let Some(dir) = app_dir() else { return };
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::write(dir.join("dictionary"), words.join("\n"));
}

/// Collapse whitespace and cap the length. Empty input is not a word.
pub(super) fn normalize_word(raw: &str) -> Option<String> {
    let word: String = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    let word: String = word.chars().take(MAX_LEN).collect();
    if word.is_empty() { None } else { Some(word) }
}

/// Insert `raw` in alphabetical order. False when it is empty or already present
/// (match ignores case, and the first spelling is the one that stays).
pub(super) fn insert_word(words: &mut Vec<String>, raw: &str) -> bool {
    let Some(word) = normalize_word(raw) else { return false };
    let key = word.to_lowercase();
    if words.iter().any(|w| w.to_lowercase() == key) {
        return false;
    }
    words.push(word);
    words.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
    true
}

pub(super) fn matches_query(word: &str, query: &str) -> bool {
    let q = query.trim().to_lowercase();
    q.is_empty() || word.to_lowercase().contains(&q)
}

impl MainWindow {
    pub(super) fn render_dictionary(&self, window: &Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx);
        let focused = self.dict_focus.is_focused(window);
        let shown: Vec<(usize, &String)> = self
            .words
            .iter()
            .enumerate()
            .filter(|(_, word)| matches_query(word, &self.dict_query))
            .collect();

        let body = if shown.is_empty() {
            let (title, detail) = if self.words.is_empty() && self.dict_query.trim().is_empty() {
                ("No words yet", "Add a name or term so dictation keeps your spelling")
            } else {
                ("Nothing matches", "Try a different search")
            };
            empty_state(t, title, detail).into_any_element()
        } else {
            div()
                .flex()
                .flex_wrap()
                .items_start()
                .gap(px(8.))
                .w_full()
                .children(shown.into_iter().map(|(index, word)| {
                    let card = self.word_card(t, index, word, cx);
                    if self.just_added.as_deref() == Some(word.as_str()) {
                        fade_in(card, ("dict-in", self.dict_nonce), 0, true)
                    } else {
                        card.into_any_element()
                    }
                }))
                .into_any_element()
        };

        page()
            .child(page_header(
                t,
                "Dictionary",
                "Names and terms dictation should spell as written",
            ))
            .child(self.dict_toolbar(t, focused, cx))
            .child(div().pt(px(16.)).child(body))
    }

    fn dict_toolbar(&self, t: Theme, focused: bool, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .gap(px(8.))
            .w_full()
            .child(self.dict_search_field(t, focused, cx))
            .child(self.add_word_button(t, cx))
    }

    fn dict_search_field(&self, t: Theme, focused: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let placeholder = self.dict_query.is_empty();
        div()
            .id("dictionary-query")
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
            .track_focus(&self.dict_focus)
            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| {
                window.focus(&this.dict_focus);
                cx.notify();
            }))
            .on_key_down(cx.listener(Self::on_dict_search_key))
            .child(
                svg()
                    .path("icons/search.svg")
                    .size(px(15.))
                    .flex_none()
                    .text_color(if focused { t.text_secondary } else { t.text_muted }),
            )
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_w(px(0.))
                    .items_center()
                    .overflow_hidden()
                    .text_size(px(13.))
                    .line_height(px(18.))
                    .when(!placeholder, |el| {
                        el.child(
                            div()
                                .text_color(t.text)
                                .when(self.dict_replace, |text| {
                                    text.bg(t.surface_active).rounded(px(3.)).px(px(2.))
                                })
                                .child(SharedString::from(self.dict_query.clone())),
                        )
                    })
                    .when(focused && !self.dict_replace, |el| {
                        el.child(div().w(px(1.5)).h(px(14.)).flex_none().bg(t.text))
                    })
                    .when(placeholder, |el| {
                        el.child(div().text_color(t.text_muted).child("Search dictionary"))
                    }),
            )
    }

    fn add_word_button(&self, t: Theme, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("add-word")
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
            .on_click(cx.listener(|this, _, window, cx| this.open_dict_dialog(window, cx)))
            .child(svg().path("icons/plus.svg").size(px(14.)).text_color(t.primary_text))
            .child("Add Word")
    }

    fn word_card(&self, t: Theme, index: usize, word: &str, cx: &mut Context<Self>) -> gpui::Stateful<gpui::Div> {
        let group: SharedString = format!("dict-{index}").into();
        div()
            .id(("dict-word", index))
            .group(group.clone())
            .flex()
            .flex_none()
            .items_center()
            .gap(px(4.))
            .h(px(36.))
            .pl(px(12.))
            .pr(px(6.))
            .rounded(px(RADIUS_SM_PX))
            .bg(t.surface)
            .border_1()
            .border_color(t.border)
            .hover(move |s| s.bg(t.surface_hover).border_color(t.border_strong))
            .child(
                div()
                    .text_size(px(13.))
                    .line_height(px(18.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(t.text)
                    .child(SharedString::from(word.to_string())),
            )
            .child(
                div()
                    .id(("dict-x", index))
                    .flex()
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
                        this.remove_word(index, cx);
                    })),
            )
    }

    // ponytail: caret stays at the end of the field. Click-to-place and IME
    // composition need an EntityInputHandler, as in GPUI's input example.
    fn on_dict_search_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if !self.dict_focus.is_focused(window) {
            return;
        }
        let ks = &event.keystroke;
        if ks.modifiers.control || ks.modifiers.platform {
            match ks.key.as_str() {
                "a" => self.dict_replace = !self.dict_query.is_empty(),
                "v" => {
                    if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
                        self.push_dict_search_text(&text);
                    }
                }
                _ => return,
            }
            cx.stop_propagation();
            cx.notify();
            return;
        }
        match ks.key.as_str() {
            "backspace" => {
                if self.dict_replace || ks.modifiers.alt {
                    self.dict_query.clear();
                } else {
                    self.dict_query.pop();
                }
                self.dict_replace = false;
            }
            "escape" => {
                if self.dict_query.is_empty() {
                    window.blur();
                } else {
                    self.dict_query.clear();
                    self.dict_replace = false;
                }
            }
            _ => {
                let typed = ks.key_char.clone().or_else(|| (ks.key.as_str() == "space").then(|| " ".to_string()));
                let Some(typed) = typed else { return };
                if typed == "\n" || typed == "\t" || typed == "\r" {
                    return;
                }
                self.push_dict_search_text(&typed);
            }
        }
        cx.stop_propagation();
        cx.notify();
    }

    fn push_dict_search_text(&mut self, extra: &str) {
        if self.dict_replace {
            self.dict_query.clear();
            self.dict_replace = false;
        }
        for ch in extra.chars() {
            if ch == '\n' || ch == '\r' || ch == '\t' {
                if !self.dict_query.is_empty() && !self.dict_query.ends_with(' ') {
                    self.dict_query.push(' ');
                }
                continue;
            }
            if self.dict_query.chars().count() >= 64 {
                break;
            }
            self.dict_query.push(ch);
        }
    }

    fn open_dict_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.dict_dialog && !self.dict_dialog_closing {
            window.focus(&self.dict_word_focus);
            return;
        }
        self.dict_draft.clear();
        self.dict_dialog_replace = false;
        self.dict_dialog_closing = false;
        self.dict_dialog = true;
        self.dict_dialog_nonce += 1;
        self.dict_dialog_epoch += 1;
        window.focus(&self.dict_word_focus);
        cx.notify();
    }

    fn dismiss_dict_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.dict_dialog || self.dict_dialog_closing {
            return;
        }
        self.dict_dialog_closing = true;
        self.dict_dialog_nonce += 1;
        self.dict_dialog_epoch += 1;
        let epoch = self.dict_dialog_epoch;
        window.blur();
        cx.notify();
        cx.spawn(async move |this, cx| {
            Timer::after(Duration::from_millis(DIALOG_MS)).await;
            let _ = this.update(cx, |this, cx| {
                if this.dict_dialog_epoch == epoch {
                    this.dict_dialog = false;
                    this.dict_dialog_closing = false;
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn confirm_dict_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let draft = self.dict_draft.clone();
        if !insert_word(&mut self.words, &draft) {
            window.focus(&self.dict_word_focus);
            cx.notify();
            return;
        }
        save_words(&self.words);
        self.just_added = normalize_word(&draft);
        self.dict_nonce += 1;
        self.dismiss_dict_dialog(window, cx);
    }

    fn remove_word(&mut self, index: usize, cx: &mut Context<Self>) {
        if index >= self.words.len() {
            return;
        }
        if self.just_added.as_deref() == Some(self.words[index].as_str()) {
            self.just_added = None;
        }
        self.words.remove(index);
        save_words(&self.words);
        cx.notify();
    }

    pub(super) fn dict_dialog(&self, t: Theme, window: &Window, cx: &mut Context<Self>) -> impl IntoElement {
        let closing = self.dict_dialog_closing;
        let nonce = self.dict_dialog_nonce;
        let panel = self.dict_dialog_panel(t, window, cx).with_animation(
            ("dict-panel", nonce),
            Animation::new(Duration::from_millis(DIALOG_MS)).with_easing(ease_out_quint()),
            move |el, d| {
                let shown = if closing { 1. - d } else { d };
                el.mt(px(16. * (1. - shown)))
            },
        );
        div()
            .id("dict-scrim")
            .absolute()
            .inset_0()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgba(0x1312115c))
            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| {
                this.dismiss_dict_dialog(window, cx);
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if event.keystroke.key == "escape" && !event.keystroke.modifiers.modified() {
                    this.dismiss_dict_dialog(window, cx);
                }
            }))
            .child(panel)
            .with_animation(
                ("dict-scrim", nonce),
                Animation::new(Duration::from_millis(DIALOG_MS)).with_easing(ease_out_quint()),
                move |el, d| {
                    let shown = if closing { 1. - d } else { d };
                    el.opacity(shown)
                },
            )
    }

    fn dict_dialog_panel(&self, t: Theme, window: &Window, cx: &mut Context<Self>) -> gpui::Stateful<gpui::Div> {
        let ready = normalize_word(&self.dict_draft).is_some();
        div()
            .id("dict-dialog")
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
                    .child("Add word"),
            )
            .child(self.dict_dialog_field(t, window, cx))
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap(px(8.))
                    .pt(px(2.))
                    .child(self.dict_dialog_cancel(t, cx))
                    .child(self.dict_dialog_confirm(t, ready, cx)),
            )
    }

    fn dict_dialog_field(&self, t: Theme, window: &Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focused = self.dict_word_focus.is_focused(window);
        let empty = self.dict_draft.is_empty();
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
                    .child("Word"),
            )
            .child(
                div()
                    .id("dict-word-field")
                    .h(px(36.))
                    .flex()
                    .items_center()
                    .px(px(12.))
                    .rounded(px(RADIUS_SM_PX))
                    .bg(t.input_bg)
                    .border_1()
                    .border_color(if focused { t.primary } else { t.border })
                    .cursor_text()
                    .track_focus(&self.dict_word_focus)
                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| {
                        cx.stop_propagation();
                        this.dict_dialog_replace = false;
                        window.focus(&this.dict_word_focus);
                        cx.notify();
                    }))
                    .on_key_down(cx.listener(Self::on_dict_dialog_key))
                    .child(
                        div()
                            .flex()
                            .flex_1()
                            .min_w(px(0.))
                            .items_center()
                            .overflow_hidden()
                            .text_size(px(13.))
                            .line_height(px(18.))
                            .when(!empty, |el| {
                                el.child(
                                    div()
                                        .text_color(t.text)
                                        .when(focused && self.dict_dialog_replace, |text| {
                                            text.bg(t.surface_active).rounded(px(3.)).px(px(2.))
                                        })
                                        .child(SharedString::from(self.dict_draft.clone())),
                                )
                            })
                            .when(focused && !self.dict_dialog_replace, |el| {
                                el.child(div().w(px(1.5)).h(px(14.)).flex_none().bg(t.text))
                            })
                            .when(empty, |el| {
                                el.child(div().text_color(t.text_muted).child("Hush Voice"))
                            }),
                    ),
            )
    }

    fn dict_dialog_cancel(&self, t: Theme, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("dict-cancel")
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
            .on_click(cx.listener(|this, _, window, cx| this.dismiss_dict_dialog(window, cx)))
            .child("Cancel")
    }

    fn dict_dialog_confirm(&self, t: Theme, ready: bool, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("dict-confirm")
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
            .on_click(cx.listener(|this, _, window, cx| this.confirm_dict_dialog(window, cx)))
            .child("Add word")
    }

    fn on_dict_dialog_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if !self.dict_word_focus.is_focused(window) {
            return;
        }
        let ks = &event.keystroke;
        if ks.modifiers.control || ks.modifiers.platform {
            match ks.key.as_str() {
                "a" => self.dict_dialog_replace = !self.dict_draft.is_empty(),
                "v" => {
                    if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
                        self.push_dict_draft_text(&text);
                    }
                }
                _ => return,
            }
            cx.stop_propagation();
            cx.notify();
            return;
        }
        match ks.key.as_str() {
            "backspace" => {
                if self.dict_dialog_replace || ks.modifiers.alt {
                    self.dict_draft.clear();
                } else {
                    self.dict_draft.pop();
                }
                self.dict_dialog_replace = false;
            }
            "enter" => self.confirm_dict_dialog(window, cx),
            "escape" => self.dismiss_dict_dialog(window, cx),
            _ => {
                let typed = ks.key_char.clone().or_else(|| (ks.key.as_str() == "space").then(|| " ".to_string()));
                let Some(typed) = typed else { return };
                if typed == "\n" || typed == "\t" || typed == "\r" {
                    return;
                }
                self.push_dict_draft_text(&typed);
            }
        }
        cx.stop_propagation();
        cx.notify();
    }

    fn push_dict_draft_text(&mut self, extra: &str) {
        if self.dict_dialog_replace {
            self.dict_draft.clear();
            self.dict_dialog_replace = false;
        }
        for ch in extra.chars() {
            if ch == '\n' || ch == '\r' || ch == '\t' {
                if !self.dict_draft.is_empty() && !self.dict_draft.ends_with(' ') {
                    self.dict_draft.push(' ');
                }
                continue;
            }
            if self.dict_draft.chars().count() >= MAX_LEN {
                break;
            }
            self.dict_draft.push(ch);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dictionary_keeps_the_spelling_you_typed() {
        let mut words = Vec::new();
        assert!(insert_word(&mut words, "  Hush   Voice "));
        assert_eq!(words, vec!["Hush Voice".to_string()]);
        assert!(!insert_word(&mut words, "hush voice"));
        assert!(insert_word(&mut words, "gpui"));
        assert_eq!(words, vec!["gpui".to_string(), "Hush Voice".to_string()]);
        assert!(matches_query("Hush Voice", " hush "));
        assert!(!matches_query("Hush Voice", "zzz"));
        assert!(!insert_word(&mut words, "   "));
    }
}
