mod chrome;
mod pages;
mod settings;
mod sidebar;

use std::time::Duration;

use gpui::{ClipboardItem, Context, SharedString, Subscription, Timer, Window, div, prelude::*, px};

use crate::models::{ModelOption, Page, TranscriptRow, sample_history};
use crate::theme::theme;

pub(crate) struct MainWindow {
    page: Page,
    models: Vec<ModelOption>,
    active_model: usize,
    hotkey: SharedString,
    history: Vec<TranscriptRow>,
    /// History row whose text was just copied (shows a check for ~1.2s).
    copied: Option<usize>,
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
            _appearance: appearance,
        }
    }

    fn set_page(&mut self, page: Page, cx: &mut Context<Self>) {
        self.page = page;
        cx.notify();
    }

    fn copy_transcript(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(row) = self.history.get(index) else { return };
        cx.write_to_clipboard(ClipboardItem::new_string(row.text.to_string()));
        self.copied = Some(index);
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
        self.active_model = index;
        cx.notify();
    }
}

impl Render for MainWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx);
        let content = match self.page {
            Page::Home => self.render_home(cx).into_any_element(),
            Page::Settings => self.render_settings_page(cx).into_any_element(),
            Page::Dictionary => self.render_dictionary(cx).into_any_element(),
            Page::Shortcuts => self.render_shortcuts(cx).into_any_element(),
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
    }
}
