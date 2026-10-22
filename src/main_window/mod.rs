mod chrome;
mod dictionary;
mod pages;
mod settings;
mod shortcuts;
mod sidebar;
mod style;

pub(crate) use dictionary::load_words;
pub(crate) use shortcuts::load_shortcuts;

use std::time::Duration;

use gpui::{
    Animation, AnimationExt, ClipboardItem, Context, FocusHandle, SharedString, Subscription, Timer,
    Window, div, ease_out_quint, prelude::*, px,
};

use crate::models::{Page, Shortcut, TranscriptRow};
use crate::overlay::audio::{MicMeter, input_device_names};
use crate::prefs::{prefs, update_prefs};
use crate::theme::theme;
use settings::SettingsTab;

pub(crate) struct MainWindow {
    page: Page,
    hotkey: SharedString,
    history: Vec<TranscriptRow>,
    /// History row whose text was just copied (shows a check for ~1.2s).
    copied: Option<usize>,
    /// Taskbar/window icon has been set (always the dark tile).
    icon_applied: bool,
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
    settings_tab: SettingsTab,
    /// Bumped on tab switch so the body cross-fades.
    tab_nonce: usize,
    /// Per-switch animation keys: [launch at login, sound cues].
    switch_nonce: [usize; 3],
    /// Model downloads in flight, by model id.
    downloads: std::collections::HashMap<&'static str, std::sync::Arc<crate::engine::Download>>,
    /// "Clear history" is waiting for a second click.
    clear_confirm: bool,
    /// Cached HKCU Run key state.
    launch_at_login: bool,
    /// Live meter while the Audio tab is open; dropped (mic released) otherwise.
    mic_test: Option<MicMeter>,
    mic_level: f32,
    mic_devices: Vec<String>,
    default_mic: Option<String>,
    mic_nonce: usize,
    style_nonce: usize,
    _appearance: Subscription,
}

impl MainWindow {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        // Re-render when Windows switches light/dark (used by the System option).
        let appearance = cx.observe_window_appearance(window, |_, _, cx| cx.notify());
        Self {
            page: Page::Home,
            hotkey: "Ctrl + Space".into(),
            history: Vec::new(),
            copied: None,
            icon_applied: false,
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
            settings_tab: SettingsTab::General,
            tab_nonce: 0,
            switch_nonce: [0; 3],
            downloads: Default::default(),
            clear_confirm: false,
            launch_at_login: crate::prefs::autostart::enabled(),
            mic_test: None,
            mic_level: 0.,
            mic_devices: Vec::new(),
            default_mic: None,
            mic_nonce: 0,
            style_nonce: 0,
            _appearance: appearance,
        }
    }

    pub(super) fn start_model_download(&mut self, m: crate::engine::ModelInfo, cx: &mut Context<Self>) {
        if self.downloads.get(m.id).is_some_and(|d| d.running()) {
            return;
        }
        let dl = crate::engine::start_download(m);
        self.downloads.insert(m.id, dl.clone());
        cx.notify();
        cx.spawn(async move |this, cx| {
            while dl.running() {
                Timer::after(Duration::from_millis(120)).await;
                let _ = this.update(cx, |_, cx| cx.notify());
            }
            let ok = dl.status.load(std::sync::atomic::Ordering::Relaxed) == 1;
            let _ = this.update(cx, |this, cx| {
                this.downloads.remove(m.id);
                // First model in: use it straight away.
                let p = prefs(cx).clone();
                if ok && crate::engine::resolve(&p.model, p.custom_model.as_deref()).is_none() {
                    this.dot_nonce += 1;
                    update_prefs(cx, |p| p.model = m.id.to_string());
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(super) fn open_model_settings(&mut self, cx: &mut Context<Self>) {
        self.set_page(Page::Settings, cx);
        self.set_settings_tab(SettingsTab::Dictation, cx);
    }

    fn set_settings_tab(&mut self, tab: SettingsTab, cx: &mut Context<Self>) {
        if self.settings_tab != tab {
            self.settings_tab = tab;
            self.tab_nonce += 1;
            if tab == SettingsTab::Audio {
                // Refresh on entry; plugging a headset in shows up next visit.
                self.mic_devices = input_device_names();
                self.default_mic = cpal_default_name();
            }
            cx.notify();
        }
    }

    /// Keep the Audio tab's test meter running only while it is on screen.
    fn tick_mic_test(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let visible = self.page == Page::Settings && self.settings_tab == SettingsTab::Audio;
        if !visible {
            self.mic_test = None;
            self.mic_level = 0.;
            return;
        }
        if self.mic_test.is_none() {
            self.mic_test = MicMeter::start(prefs(cx).mic.as_deref());
        }
        if let Some(meter) = &self.mic_test {
            // The test meter never transcribes: drop what it captured.
            let _ = meter.take_samples();
            // -60 dBFS .. -12 dBFS -> 0..1, quick rise, gentle fall.
            let db = 20. * meter.level().max(1e-6).log10();
            let target = ((db + 60.) / 48.).clamp(0., 1.);
            let k = if target > self.mic_level { 0.5 } else { 0.12 };
            self.mic_level += (target - self.mic_level) * k;
            window.request_animation_frame();
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

}

impl Render for MainWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.icon_applied {
            crate::app_icon::apply(window);
            self.icon_applied = true;
        }
        self.tick_mic_test(window, cx);
        self.history = crate::history::rows(cx);
        let t = theme(cx);
        let content = match self.page {
            Page::Style => self.render_style(cx).into_any_element(),
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

fn cpal_default_name() -> Option<String> {
    use cpal::traits::{DeviceTrait, HostTrait};
    cpal::default_host().default_input_device()?.name().ok()
}
