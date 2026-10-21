mod chrome;
mod dictionary;
mod pages;
mod settings;
mod shortcuts;
mod sidebar;

use std::time::Duration;

use gpui::{
    Animation, AnimationExt, ClipboardItem, Context, FocusHandle, SharedString, Subscription, Timer,
    Window, div, ease_out_quint, prelude::*, px,
};

use crate::models::{ModelOption, Page, Shortcut, TranscriptRow, sample_history};
use crate::theme::theme;

pub(crate) struct MainWindow {
    page: Page,
    models: Vec<ModelOption>,
    active_model: usize,
    hotkey: SharedString,
    history: Vec<TranscriptRow>,
    /// History row whose text was just copied (shows a check for ~1.2s).
    copied: Option<usize>,
    /// Theme the taskbar icon was last set for (dark = true).
    icon_dark: Option<bool>,
    /// Bumped on each copy so the check icon replays its pop-in.
    copy_nonce: usize,
    /// Number of page switches; 0 means first show (drives entrance stagger).
    nav_count: usize,
    /// Bumped when the selected model changes so the radio dot scales in.
    dot_nonce: usize,
    shortcuts: Vec<Shortcut>,
    shortcut_query: String,
    /// Next typed character replaces the shortcut search field (Ctrl+A).
    shortcut_replace: bool,
    shortcut_focus: FocusHandle,
    /// Trigger that just landed in the grid, so its card can rise in once.
    shortcut_added: Option<String>,
    shortcut_nonce: usize,
    shortcut_dialog: bool,
    dialog_closing: bool,
    dialog_epoch: usize,
    dialog_nonce: usize,
    dialog_word: String,
    dialog_shortcut: String,
    dialog_replace: bool,
    word_focus: FocusHandle,
    dialog_shortcut_focus: FocusHandle,
    /// Custom spellings. Speech models read the saved list when a token is uncertain.
    words: Vec<String>,
    dict_query: String,
    /// Next typed character replaces the dictionary query (Ctrl+A).
    dict_replace: bool,
    dict_focus: FocusHandle,
    /// Word that just landed in the grid, so its card can rise in once.
    just_added: Option<String>,
    dict_nonce: usize,
    dict_dialog: bool,
    dict_dialog_closing: bool,
    dict_dialog_epoch: usize,
    dict_dialog_nonce: usize,
    dict_draft: String,
    dict_dialog_replace: bool,
    dict_word_focus: FocusHandle,
    _appearance: Subscription,
}

impl MainWindow {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        // Re-render when Windows switches light/dark (used by the System option).
        let appearance = cx.observe_window_appearance(window, |_, _, cx| cx.notify());
        Self {
            page: Page::Home,
            models: vec![
                ModelOption {
                    id: "whisper-local",
                    name: "Whisper Local".into(),
                    detail: "On-device \u{00b7} fast \u{00b7} private".into(),
                },
                ModelOption {
                    id: "whisper-large",
                    name: "Whisper Large".into(),
                    detail: "Higher accuracy \u{00b7} slower".into(),
                },
                ModelOption {
                    id: "cloud-stt",
                    name: "Cloud STT".into(),
                    detail: "Lowest latency \u{00b7} requires network".into(),
                },
            ],
            active_model: 0,
            hotkey: "Ctrl + Space".into(),
            history: sample_history(),
            copied: None,
            icon_dark: None,
            copy_nonce: 0,
            nav_count: 0,
            dot_nonce: 0,
            shortcuts: shortcuts::load_shortcuts(),
            shortcut_query: String::new(),
            shortcut_replace: false,
            shortcut_focus: cx.focus_handle(),
            shortcut_added: None,
            shortcut_nonce: 0,
            shortcut_dialog: false,
            dialog_closing: false,
            dialog_epoch: 0,
            dialog_nonce: 0,
            dialog_word: String::new(),
            dialog_shortcut: String::new(),
            dialog_replace: false,
            word_focus: cx.focus_handle(),
            dialog_shortcut_focus: cx.focus_handle(),
            words: dictionary::load_words(),
            dict_query: String::new(),
            dict_replace: false,
            dict_focus: cx.focus_handle(),
            just_added: None,
            dict_nonce: 0,
            dict_dialog: false,
            dict_dialog_closing: false,
            dict_dialog_epoch: 0,
            dict_dialog_nonce: 0,
            dict_draft: String::new(),
            dict_dialog_replace: false,
            dict_word_focus: cx.focus_handle(),
            _appearance: appearance,
        }
    }

    fn set_page(&mut self, page: Page, cx: &mut Context<Self>) {
        if self.page != page {
            self.nav_count += 1;
        }
        self.page = page;
        cx.notify();
    }

    fn copy_transcript(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(row) = self.history.get(index) else { return };
        cx.write_to_clipboard(ClipboardItem::new_string(row.text.to_string()));
        self.copied = Some(index);
        self.copy_nonce += 1;
        cx.notify();
        cx.spawn(async move |this, cx| {
            Timer::after(Duration::from_millis(1200)).await;
            let _ = this.update(cx, |this, cx| {
                if this.copied == Some(index) {
                    this.copied = None;
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn select_model(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.active_model != index {
            self.dot_nonce += 1;
        }
        self.active_model = index;
        cx.notify();
    }
}

impl Render for MainWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let dark = crate::theme::theme_mode(cx) == crate::theme::ThemeMode::Dark;
        if self.icon_dark != Some(dark) {
            crate::app_icon::apply(window, dark);
            self.icon_dark = Some(dark);
        }
        let t = theme(cx);
        let content = match self.page {
            Page::Home => self.render_home(cx).into_any_element(),
            Page::Settings => self.render_settings_page(cx).into_any_element(),
            Page::Dictionary => self.render_dictionary(window, cx).into_any_element(),
            Page::Shortcuts => self.render_shortcuts(window, cx).into_any_element(),
        };
        // Page switch: short fade + 6px rise. Skipped on first show, where the
        // Home cards/rows run their own staggered entrance instead.
        let content = if self.nav_count == 0 {
            content
        } else {
            div()
                .w_full()
                .child(content)
                .with_animation(
                    ("page", self.nav_count),
                    Animation::new(Duration::from_millis(180)).with_easing(ease_out_quint()),
                    |el, d| el.opacity(d).mt(px(6. * (1. - d))),
                )
                .into_any_element()
        };

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(t.bg)
            .text_color(t.text)
            .font_family(".SystemUIFont")
            .child(self.render_titlebar(cx))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h(px(0.))
                    .w_full()
                    .child(self.render_sidebar(cx))
                    .child(
                        // Inset content panel.
                        div()
                            .flex_1()
                            .h_full()
                            .pr(px(8.))
                            .pb(px(8.))
                            .pt(px(2.))
                            .child(
                                div()
                                    .id("main-content")
                                    .size_full()
                                    .rounded(px(12.))
                                    .bg(t.panel)
                                    .px(px(28.))
                                    .pt(px(24.))
                                    .pb(px(32.))
                                    .overflow_y_scroll()
                                    .child(content),
                            ),
                    ),
            )
            .when(self.shortcut_dialog, |el| el.child(self.shortcut_dialog(t, window, cx)))
            .when(self.dict_dialog, |el| el.child(self.dict_dialog(t, window, cx)))
    }
}
