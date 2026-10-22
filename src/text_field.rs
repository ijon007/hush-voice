//! Single-line text field: caret, selection, word motion, and clipboard.
//! GPUI's divs don't do this; every in-app input shares this editor.

use std::ops::Deref;

use gpui::{
    App, ClipboardItem, Div, IntoElement, Keystroke, ParentElement, SharedString, Styled, div,
    prelude::*, px,
};

use crate::theme::Theme;

#[derive(Clone, Debug)]
pub(crate) struct Field {
    text: String,
    /// Caret as a character index (the active end of the selection).
    caret: usize,
    /// Other end of the selection. Equal to `caret` when nothing is selected.
    anchor: usize,
    max: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Space,
    Word,
    Other,
}

enum Action {
    Ignored,
    Changed,
    Copy(String),
    Cut(String),
    Paste,
}

impl Field {
    pub(crate) fn new(max: usize) -> Self {
        Self { text: String::new(), caret: 0, anchor: 0, max }
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.text
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    pub(crate) fn clear(&mut self) {
        self.text.clear();
        self.caret = 0;
        self.anchor = 0;
    }

    fn len(&self) -> usize {
        self.text.chars().count()
    }

    fn sel(&self) -> (usize, usize) {
        if self.caret <= self.anchor {
            (self.caret, self.anchor)
        } else {
            (self.anchor, self.caret)
        }
    }

    fn has_sel(&self) -> bool {
        self.caret != self.anchor
    }

    fn set_caret(&mut self, i: usize, select: bool) {
        self.caret = i.min(self.len());
        if !select {
            self.anchor = self.caret;
        }
    }

    fn delete_range(&mut self, from: usize, to: usize) {
        if from >= to {
            return;
        }
        let mut chars: Vec<char> = self.text.chars().collect();
        chars.drain(from..to);
        self.text = chars.into_iter().collect();
        self.caret = from;
        self.anchor = from;
    }

    fn delete_sel(&mut self) -> bool {
        if !self.has_sel() {
            return false;
        }
        let (a, b) = self.sel();
        self.delete_range(a, b);
        true
    }

    fn selected(&self) -> String {
        let (a, b) = self.sel();
        self.text.chars().skip(a).take(b - a).collect()
    }

    fn insert_plain(&mut self, extra: &str) {
        self.delete_sel();
        let mut chars: Vec<char> = self.text.chars().collect();
        for ch in extra.chars() {
            let ch = match ch {
                '\n' | '\r' | '\t' => {
                    if self.caret > 0 && chars.get(self.caret - 1) != Some(&' ') {
                        ' '
                    } else {
                        continue;
                    }
                }
                c => c,
            };
            if chars.len() >= self.max {
                break;
            }
            chars.insert(self.caret, ch);
            self.caret += 1;
        }
        self.text = chars.into_iter().collect();
        self.anchor = self.caret;
    }

    fn backspace(&mut self) {
        if self.delete_sel() {
            return;
        }
        if self.caret == 0 {
            return;
        }
        self.delete_range(self.caret - 1, self.caret);
    }

    fn delete_forward(&mut self) {
        if self.delete_sel() {
            return;
        }
        if self.caret >= self.len() {
            return;
        }
        self.delete_range(self.caret, self.caret + 1);
    }

    fn delete_word_back(&mut self) {
        if self.delete_sel() {
            return;
        }
        let dest = prev_word(&self.text, self.caret);
        self.delete_range(dest, self.caret);
    }

    fn delete_word_forward(&mut self) {
        if self.delete_sel() {
            return;
        }
        let dest = next_word(&self.text, self.caret);
        self.delete_range(self.caret, dest);
    }

    fn move_left(&mut self, select: bool, word: bool) {
        if !select && self.has_sel() && !word {
            let (a, _) = self.sel();
            self.set_caret(a, false);
            return;
        }
        let dest = if word { prev_word(&self.text, self.caret) } else { self.caret.saturating_sub(1) };
        self.set_caret(dest, select);
    }

    fn move_right(&mut self, select: bool, word: bool) {
        if !select && self.has_sel() && !word {
            let (_, b) = self.sel();
            self.set_caret(b, false);
            return;
        }
        let dest = if word { next_word(&self.text, self.caret) } else { self.caret.saturating_add(1) };
        self.set_caret(dest, select);
    }

    fn select_all(&mut self) {
        self.anchor = 0;
        self.caret = self.len();
    }

    fn handle(&mut self, key: &str, ctrl: bool, shift: bool, key_char: Option<&str>) -> Action {
        let word = ctrl;
        match key {
            "enter" | "tab" | "escape" | "up" | "down" => return Action::Ignored,
            "left" => {
                self.move_left(shift, word);
                return Action::Changed;
            }
            "right" => {
                self.move_right(shift, word);
                return Action::Changed;
            }
            "home" => {
                self.set_caret(0, shift);
                return Action::Changed;
            }
            "end" => {
                self.set_caret(self.len(), shift);
                return Action::Changed;
            }
            "backspace" => {
                if ctrl {
                    self.delete_word_back();
                } else {
                    self.backspace();
                }
                return Action::Changed;
            }
            "delete" => {
                if ctrl {
                    self.delete_word_forward();
                } else {
                    self.delete_forward();
                }
                return Action::Changed;
            }
            _ => {}
        }
        if ctrl {
            return match key {
                "a" => {
                    self.select_all();
                    Action::Changed
                }
                "c" => Action::Copy(self.selected()),
                "x" => Action::Cut(self.selected()),
                "v" => Action::Paste,
                _ => Action::Ignored,
            };
        }
        let typed = key_char.map(str::to_string).or_else(|| (key == "space").then(|| " ".to_string()));
        let Some(typed) = typed else { return Action::Ignored };
        if typed == "\n" || typed == "\t" || typed == "\r" {
            return Action::Ignored;
        }
        self.insert_plain(&typed);
        Action::Changed
    }
}

impl Deref for Field {
    type Target = str;
    fn deref(&self) -> &str {
        &self.text
    }
}

fn kind(c: char) -> Kind {
    if c.is_whitespace() {
        Kind::Space
    } else if c.is_alphanumeric() || c == '_' {
        Kind::Word
    } else {
        Kind::Other
    }
}

fn prev_word(text: &str, caret: usize) -> usize {
    let chars: Vec<char> = text.chars().collect();
    let mut i = caret.min(chars.len());
    if i == 0 {
        return 0;
    }
    while i > 0 && kind(chars[i - 1]) == Kind::Space {
        i -= 1;
    }
    if i == 0 {
        return 0;
    }
    let k = kind(chars[i - 1]);
    while i > 0 && kind(chars[i - 1]) == k {
        i -= 1;
    }
    i
}

fn next_word(text: &str, caret: usize) -> usize {
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    let mut i = caret.min(n);
    if i >= n {
        return n;
    }
    let k = kind(chars[i]);
    while i < n && kind(chars[i]) == k {
        i += 1;
    }
    while i < n && kind(chars[i]) == Kind::Space {
        i += 1;
    }
    i
}

/// Apply a key to `field`. True when the field consumed it (caller should stop propagation).
pub(crate) fn apply_key(field: &mut Field, ks: &Keystroke, cx: &mut App) -> bool {
    let ctrl = ks.modifiers.control || ks.modifiers.platform;
    if ctrl && ks.modifiers.alt {
        return false;
    }
    match field.handle(ks.key.as_str(), ctrl, ks.modifiers.shift, ks.key_char.as_deref()) {
        Action::Ignored => false,
        Action::Changed => true,
        Action::Copy(text) => {
            if !text.is_empty() {
                cx.write_to_clipboard(ClipboardItem::new_string(text));
            }
            true
        }
        Action::Cut(text) => {
            if !text.is_empty() {
                cx.write_to_clipboard(ClipboardItem::new_string(text));
                field.delete_sel();
            }
            true
        }
        Action::Paste => {
            if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
                field.insert_plain(&text);
            }
            true
        }
    }
}

fn caret_bar(t: Theme) -> Div {
    div().w(px(1.5)).h(px(14.)).flex_none().bg(t.text)
}

fn chunk(text: String, t: Theme, selected: bool) -> Div {
    div()
        .text_color(t.text)
        .when(selected, |el| el.bg(t.surface_active).rounded(px(3.)))
        .child(SharedString::from(text))
}

/// Caret, selection highlight, or placeholder. Used by every in-app input.
pub(crate) fn paint(field: &Field, focused: bool, placeholder: &'static str, t: Theme) -> impl IntoElement {
    let empty = field.is_empty();
    div()
        .flex()
        .flex_1()
        .min_w(px(0.))
        .items_center()
        .overflow_hidden()
        .text_size(px(13.))
        .line_height(px(18.))
        .when(empty, |el| {
            el.when(focused, |el| el.child(caret_bar(t)))
                .child(div().text_color(t.text_muted).child(placeholder))
        })
        .when(!empty, |el| {
            let n = field.len();
            let (a, b) = field.sel();
            let before: String = field.text.chars().take(a).collect();
            let sel: String = field.text.chars().skip(a).take(b - a).collect();
            let after: String = field.text.chars().skip(b).take(n - b).collect();
            let caret_at = if focused { Some(field.caret) } else { None };
            el.when(!before.is_empty(), |el| el.child(chunk(before, t, false)))
                .when(caret_at == Some(a), |el| el.child(caret_bar(t)))
                .when(!sel.is_empty(), |el| el.child(chunk(sel, t, focused)))
                .when(caret_at == Some(b) && a != b, |el| el.child(caret_bar(t)))
                .when(!after.is_empty(), |el| el.child(chunk(after, t, false)))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn type_text(field: &mut Field, s: &str) {
        for ch in s.chars() {
            let key = if ch == ' ' { "space" } else { "x" };
            let owned = ch.to_string();
            field.handle(key, false, false, Some(&owned));
        }
    }

    #[test]
    fn word_keys_match_windows() {
        let mut f = Field::new(64);
        type_text(&mut f, "hello world");
        assert_eq!(f.as_str(), "hello world");
        assert_eq!(f.caret, 11);

        f.handle("left", true, false, None);
        assert_eq!(f.caret, 6, "ctrl+left lands on w");
        f.handle("left", true, false, None);
        assert_eq!(f.caret, 0, "ctrl+left lands on h");

        f.handle("right", true, true, None);
        assert_eq!(f.sel(), (0, 6));
        assert_eq!(f.selected(), "hello ");

        f.handle("end", false, false, None);
        f.handle("backspace", true, false, None);
        assert_eq!(f.as_str(), "hello ");

        f.handle("backspace", true, false, None);
        assert_eq!(f.as_str(), "");

        type_text(&mut f, "one two three");
        f.handle("a", true, false, None);
        assert_eq!(f.sel(), (0, 13));
        f.handle("backspace", false, false, None);
        assert_eq!(f.as_str(), "");

        type_text(&mut f, "alpha beta");
        f.handle("left", false, true, None);
        f.handle("left", false, true, None);
        f.handle("left", false, true, None);
        f.handle("left", false, true, None);
        assert_eq!(f.selected(), "beta");
        f.handle("x", false, false, Some("x"));
        assert_eq!(f.as_str(), "alpha x");
    }

    #[test]
    fn arrows_collapse_then_move() {
        let mut f = Field::new(64);
        type_text(&mut f, "abcd");
        f.handle("a", true, false, None);
        f.handle("left", false, false, None);
        assert_eq!(f.caret, 0);
        assert!(!f.has_sel());
        f.handle("a", true, false, None);
        f.handle("right", false, false, None);
        assert_eq!(f.caret, 4);
        assert!(!f.has_sel());
    }
}
