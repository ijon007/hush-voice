mod chrome;
mod pages;
mod settings;
mod sidebar;

use gpui::{Context, SharedString, Subscription, Window, div, prelude::*, px};

use crate::models::{ModelOption, Page, TranscriptRow, sample_history};
use crate::theme::theme;

pub(crate) struct MainWindow {
    page: Page,
    models: Vec<ModelOption>,
    active_model: usize,
    hotkey: SharedString,
    history: Vec<TranscriptRow>,
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
            _appearance: appearance,
        }
    }

    fn set_page(&mut self, page: Page, cx: &mut Context<Self>) {
        self.page = page;
        cx.notify();
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
                                    .px(px(36.))
                                    .pt(px(28.))
                                    .pb(px(32.))
                                    .overflow_y_scroll()
                                    .child(content),
                            ),
                    ),
            )
    }
}
