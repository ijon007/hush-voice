mod chrome;
mod history;
mod settings;

use gpui::{Context, SharedString, Window, div, prelude::*, px};

use crate::models::{ModelOption, Tab, TranscriptRow, sample_history};
use crate::theme::{BG, TEXT};

pub(crate) struct MainWindow {
    tab: Tab,
    models: Vec<ModelOption>,
    active_model: usize,
    hotkey: SharedString,
    history: Vec<TranscriptRow>,
}

impl MainWindow {
    pub(crate) fn new() -> Self {
        Self {
            tab: Tab::History,
            models: vec![
                ModelOption {
                    id: "whisper-local",
                    name: "Whisper Local".into(),
                    detail: "On-device · fast · private".into(),
                },
                ModelOption {
                    id: "whisper-large",
                    name: "Whisper Large".into(),
                    detail: "Higher accuracy · slower".into(),
                },
                ModelOption {
                    id: "cloud-stt",
                    name: "Cloud STT".into(),
                    detail: "Lowest latency · requires network".into(),
                },
            ],
            active_model: 0,
            hotkey: "Ctrl + Space".into(),
            history: sample_history(),
        }
    }

    fn set_tab(&mut self, tab: Tab, cx: &mut Context<Self>) {
        self.tab = tab;
        cx.notify();
    }

    fn select_model(&mut self, index: usize, cx: &mut Context<Self>) {
        self.active_model = index;
        cx.notify();
    }
}

impl Render for MainWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(BG)
            .text_color(TEXT)
            .font_family(".SystemUIFont")
            .child(self.render_titlebar())
            .child(self.render_header(cx))
            .child(
                div()
                    .id("main-content")
                    .flex_1()
                    .w_full()
                    .px(px(28.))
                    .pb(px(28.))
                    .pt(px(8.))
                    .overflow_y_scroll()
                    .child(match self.tab {
                        Tab::Settings => self.render_settings(cx).into_any_element(),
                        Tab::History => self.render_history().into_any_element(),
                    }),
            )
    }
}
