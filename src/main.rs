mod theme;

use gpui::{
    App, Application, Bounds, Context, FontWeight, SharedString, Window, WindowBounds,
    WindowOptions, div, prelude::*, px, size,
};
use theme::{
    BG, BORDER, BORDER_STRONG, PRIMARY, PRIMARY_ACTIVE, PRIMARY_TEXT, RADIUS_PX, SURFACE,
    SURFACE_ACTIVE, SURFACE_HOVER, TEXT, TEXT_MUTED, TEXT_SECONDARY,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Settings,
    History,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum InjectionMode {
    SendInput,
    ClipboardPaste,
}

#[derive(Clone)]
struct ModelOption {
    id: &'static str,
    name: SharedString,
    detail: SharedString,
}

#[derive(Clone)]
struct TranscriptRow {
    text: SharedString,
    model: SharedString,
    when: SharedString,
    duration: SharedString,
}

struct MainWindow {
    tab: Tab,
    models: Vec<ModelOption>,
    active_model: usize,
    hotkey: SharedString,
    injection: InjectionMode,
    history: Vec<TranscriptRow>,
    history_filter: Option<&'static str>,
}

impl MainWindow {
    fn new() -> Self {
        Self {
            tab: Tab::Settings,
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
            injection: InjectionMode::ClipboardPaste,
            history: sample_history(),
            history_filter: None,
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

    fn set_injection(&mut self, mode: InjectionMode, cx: &mut Context<Self>) {
        self.injection = mode;
        cx.notify();
    }

    fn set_history_filter(&mut self, filter: Option<&'static str>, cx: &mut Context<Self>) {
        self.history_filter = filter;
        cx.notify();
    }

    fn filtered_history(&self) -> impl Iterator<Item = &TranscriptRow> {
        let filter = self.history_filter;
        self.history.iter().filter(move |row| match filter {
            None => true,
            Some(model) => row.model.as_ref() == model,
        })
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
            .child(self.render_chrome(cx))
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .px(px(40.))
                    .pb(px(40.))
                    .pt(px(8.))
                    .overflow_hidden()
                    .child(match self.tab {
                        Tab::Settings => self.render_settings(cx).into_any_element(),
                        Tab::History => self.render_history(cx).into_any_element(),
                    }),
            )
    }
}

impl MainWindow {
    fn render_chrome(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .justify_between()
            .w_full()
            .px(px(40.))
            .pt(px(28.))
            .pb(px(24.))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .child(
                        div()
                            .text_size(px(22.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("hush-voice"),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(TEXT_MUTED)
                            .child("Push-to-talk transcription"),
                    ),
            )
            .child(self.render_tabs(cx))
    }

    fn render_tabs(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .p(px(4.))
            .gap(px(4.))
            .rounded(px(RADIUS_PX))
            .bg(SURFACE)
            .border_1()
            .border_color(BORDER)
            .child(self.tab_button("Settings", Tab::Settings, cx))
            .child(self.tab_button("History", Tab::History, cx))
    }

    fn tab_button(
        &self,
        label: &'static str,
        tab: Tab,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selected = self.tab == tab;
        div()
            .id(label)
            .px(px(16.))
            .py(px(8.))
            .rounded(px(RADIUS_PX - 2.))
            .text_size(px(13.))
            .font_weight(if selected {
                FontWeight::MEDIUM
            } else {
                FontWeight::NORMAL
            })
            .text_color(if selected { TEXT } else { TEXT_SECONDARY })
            .when(selected, |el| el.bg(SURFACE_HOVER))
            .hover(|s| {
                if selected {
                    s
                } else {
                    s.bg(SURFACE_ACTIVE).text_color(TEXT)
                }
            })
            .active(|s| s.opacity(0.85))
            .cursor_pointer()
            .child(label)
            .on_click(cx.listener(move |this, _, _, cx| this.set_tab(tab, cx)))
    }

    fn render_settings(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap(px(36.))
            .size_full()
            .child(section_label("Model"))
            .child(self.render_model_list(cx))
            .child(section_label("Hotkey"))
            .child(self.render_hotkey())
            .child(section_label("Injection"))
            .child(self.render_injection(cx))
            .child(
                div()
                    .flex()
                    .justify_end()
                    .pt(px(12.))
                    .child(primary_button("Save changes", "save-settings")),
            )
    }

    fn render_model_list(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut rows = Vec::new();
        for (index, model) in self.models.iter().enumerate() {
            rows.push(self.model_row(index, model, cx).into_any_element());
        }
        div()
            .flex()
            .flex_col()
            .gap(px(10.))
            .children(rows)
    }

    fn model_row(
        &self,
        index: usize,
        model: &ModelOption,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selected = self.active_model == index;
        div()
            .id(model.id)
            .flex()
            .items_center()
            .justify_between()
            .px(px(18.))
            .py(px(16.))
            .rounded(px(RADIUS_PX))
            .bg(SURFACE)
            .border_1()
            .border_color(if selected { BORDER_STRONG } else { BORDER })
            .hover(|s| s.bg(SURFACE_HOVER))
            .active(|s| s.bg(SURFACE_ACTIVE).opacity(0.95))
            .cursor_pointer()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .child(
                        div()
                            .text_size(px(15.))
                            .font_weight(FontWeight::MEDIUM)
                            .child(model.name.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(TEXT_MUTED)
                            .child(model.detail.clone()),
                    ),
            )
            .child(selection_dot(selected))
            .on_click(cx.listener(move |this, _, _, cx| this.select_model(index, cx)))
    }

    fn render_hotkey(&self) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .justify_between()
            .px(px(18.))
            .py(px(16.))
            .rounded(px(RADIUS_PX))
            .bg(SURFACE)
            .border_1()
            .border_color(BORDER)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .child(
                        div()
                            .text_size(px(15.))
                            .font_weight(FontWeight::MEDIUM)
                            .child("Push to talk"),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(TEXT_MUTED)
                            .child("Hold to record, release to transcribe"),
                    ),
            )
            .child(
                div()
                    .px(px(12.))
                    .py(px(7.))
                    .rounded(px(8.))
                    .bg(BG)
                    .border_1()
                    .border_color(BORDER_STRONG)
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(TEXT)
                    .child(self.hotkey.clone()),
            )
    }

    fn render_injection(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .p(px(4.))
            .gap(px(4.))
            .rounded(px(RADIUS_PX))
            .bg(SURFACE)
            .border_1()
            .border_color(BORDER)
            .child(self.injection_chip(
                "Clipboard + paste",
                InjectionMode::ClipboardPaste,
                cx,
            ))
            .child(self.injection_chip("SendInput", InjectionMode::SendInput, cx))
    }

    fn injection_chip(
        &self,
        label: &'static str,
        mode: InjectionMode,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selected = self.injection == mode;
        div()
            .id(label)
            .flex_1()
            .px(px(14.))
            .py(px(10.))
            .rounded(px(RADIUS_PX - 2.))
            .text_size(px(13.))
            .font_weight(if selected {
                FontWeight::MEDIUM
            } else {
                FontWeight::NORMAL
            })
            .text_color(if selected { PRIMARY_TEXT } else { TEXT_SECONDARY })
            .when(selected, |el| el.bg(PRIMARY))
            .hover(|s| {
                if selected {
                    s.bg(PRIMARY_ACTIVE)
                } else {
                    s.bg(SURFACE_HOVER).text_color(TEXT)
                }
            })
            .active(|s| s.opacity(0.9))
            .cursor_pointer()
            .child(label)
            .on_click(cx.listener(move |this, _, _, cx| this.set_injection(mode, cx)))
    }

    fn render_history(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let rows: Vec<_> = self.filtered_history().cloned().collect();

        div()
            .flex()
            .flex_col()
            .gap(px(20.))
            .size_full()
            .child(self.render_history_toolbar(cx))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .flex_1()
                    .overflow_hidden()
                    .children(if rows.is_empty() {
                        vec![empty_history().into_any_element()]
                    } else {
                        rows.into_iter()
                            .map(|row| history_row(row).into_any_element())
                            .collect()
                    }),
            )
    }

    fn render_history_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap(px(14.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .px(px(16.))
                    .h(px(42.))
                    .rounded(px(RADIUS_PX))
                    .bg(SURFACE)
                    .border_1()
                    .border_color(BORDER)
                    .child(
                        div()
                            .text_size(px(14.))
                            .text_color(TEXT_MUTED)
                            .child("Search transcripts…"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap(px(8.))
                    .child(self.filter_chip("All", None, cx))
                    .child(self.filter_chip("Whisper Local", Some("Whisper Local"), cx))
                    .child(self.filter_chip("Cloud STT", Some("Cloud STT"), cx)),
            )
    }

    fn filter_chip(
        &self,
        label: &'static str,
        filter: Option<&'static str>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selected = self.history_filter == filter;
        div()
            .id(label)
            .px(px(12.))
            .py(px(6.))
            .rounded(px(8.))
            .text_size(px(12.))
            .font_weight(FontWeight::MEDIUM)
            .text_color(if selected { PRIMARY_TEXT } else { TEXT_SECONDARY })
            .bg(if selected { PRIMARY } else { SURFACE })
            .border_1()
            .border_color(if selected { PRIMARY } else { BORDER })
            .hover(|s| {
                if selected {
                    s.bg(PRIMARY_ACTIVE)
                } else {
                    s.bg(SURFACE_HOVER).text_color(TEXT)
                }
            })
            .active(|s| s.opacity(0.9))
            .cursor_pointer()
            .child(label)
            .on_click(cx.listener(move |this, _, _, cx| this.set_history_filter(filter, cx)))
    }
}

fn section_label(text: &'static str) -> impl IntoElement {
    div()
        .text_size(px(12.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(TEXT_MUTED)
        .child(text.to_uppercase())
}

fn selection_dot(selected: bool) -> impl IntoElement {
    div()
        .size(px(16.))
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
                    .child(
                        div()
                            .size(px(6.))
                            .rounded_full()
                            .bg(PRIMARY_TEXT),
                    ),
            )
        })
}

fn primary_button(label: &'static str, id: &'static str) -> impl IntoElement {
    div()
        .id(id)
        .px(px(18.))
        .py(px(10.))
        .rounded(px(RADIUS_PX))
        .bg(PRIMARY)
        .text_color(PRIMARY_TEXT)
        .text_size(px(13.))
        .font_weight(FontWeight::MEDIUM)
        .hover(|s| s.bg(PRIMARY_ACTIVE))
        .active(|s| s.opacity(0.92))
        .cursor_pointer()
        .child(label)
}

fn history_row(row: TranscriptRow) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap(px(10.))
        .px(px(18.))
        .py(px(18.))
        .rounded(px(RADIUS_PX))
        .bg(SURFACE)
        .border_1()
        .border_color(BORDER)
        .hover(|s| s.bg(SURFACE_HOVER).border_color(BORDER_STRONG))
        .child(
            div()
                .text_size(px(14.))
                .text_color(TEXT)
                .line_height(px(22.))
                .child(row.text),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(12.))
                .child(meta_pill(row.model))
                .child(meta_text(row.duration))
                .child(meta_text(row.when)),
        )
}

fn meta_pill(text: SharedString) -> impl IntoElement {
    div()
        .px(px(8.))
        .py(px(3.))
        .rounded(px(6.))
        .bg(BG)
        .border_1()
        .border_color(BORDER)
        .text_size(px(11.))
        .text_color(TEXT_SECONDARY)
        .child(text)
}

fn meta_text(text: SharedString) -> impl IntoElement {
    div()
        .text_size(px(12.))
        .text_color(TEXT_MUTED)
        .child(text)
}

fn empty_history() -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(8.))
        .py(px(80.))
        .child(
            div()
                .text_size(px(15.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(TEXT_SECONDARY)
                .child("No transcripts yet"),
        )
        .child(
            div()
                .text_size(px(13.))
                .text_color(TEXT_MUTED)
                .child("Hold your hotkey to capture the first one"),
        )
}

fn sample_history() -> Vec<TranscriptRow> {
    vec![
        TranscriptRow {
            text: "Schedule a design review for Thursday afternoon.".into(),
            model: "Whisper Local".into(),
            when: "Today · 4:12 PM".into(),
            duration: "2.4s".into(),
        },
        TranscriptRow {
            text: "Remind me to send the hush-voice build notes.".into(),
            model: "Cloud STT".into(),
            when: "Today · 2:01 PM".into(),
            duration: "1.8s".into(),
        },
        TranscriptRow {
            text: "Draft a calm reply declining the meeting.".into(),
            model: "Whisper Local".into(),
            when: "Yesterday · 9:40 AM".into(),
            duration: "3.1s".into(),
        },
        TranscriptRow {
            text: "Open the settings panel and switch injection mode.".into(),
            model: "Whisper Large".into(),
            when: "Mon · 6:18 PM".into(),
            duration: "2.0s".into(),
        },
    ]
}

fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(880.), px(640.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(gpui::TitlebarOptions {
                    title: Some("hush-voice".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |_, cx| cx.new(|_| MainWindow::new()),
        )
        .unwrap();

        cx.activate(true);
    });
}
