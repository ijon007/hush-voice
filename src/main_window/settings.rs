use std::time::Duration;

use gpui::{
    Animation, AnimationExt, AnyElement, Context, FontWeight, SharedString, div, prelude::*, px,
    svg,
};

use crate::main_window::MainWindow;
use crate::models::{ModelOption, Page};
use crate::prefs::{OverlayPosition, autostart, prefs, update_prefs};
use crate::theme::{RADIUS_PX, RADIUS_SM_PX, Theme, ThemeMode, set_theme_mode, theme, theme_mode};
use crate::widgets::{card, ease_strong, section_label, selection_dot};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum SettingsTab {
    General,
    Audio,
    Dictation,
    Shortcuts,
    Appearance,
}

impl SettingsTab {
    const ALL: [SettingsTab; 5] = [
        SettingsTab::General,
        SettingsTab::Audio,
        SettingsTab::Dictation,
        SettingsTab::Shortcuts,
        SettingsTab::Appearance,
    ];

    fn label(self) -> &'static str {
        match self {
            SettingsTab::General => "General",
            SettingsTab::Audio => "Audio",
            SettingsTab::Dictation => "Dictation",
            SettingsTab::Shortcuts => "Shortcuts",
            SettingsTab::Appearance => "Appearance",
        }
    }
}

/// Section title + grouped rows on one surface, hairlines between rows.
fn group(t: Theme, label: &'static str, rows: Vec<AnyElement>) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap(px(8.))
        .child(section_label(t, label))
        .child(card(t).children(rows))
}

fn row_text(t: Theme, title: &'static str, detail: impl Into<SharedString>) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .flex_1()
        .min_w(px(0.))
        .gap(px(2.))
        .child(
            div()
                .text_size(px(14.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(t.text)
                .child(title),
        )
        .child(div().text_size(px(12.)).text_color(t.text_muted).child(detail.into()))
}

/// Label + description on the left, control on the right.
fn setting_row(
    t: Theme,
    title: &'static str,
    detail: impl Into<SharedString>,
    control: impl IntoElement,
    divider: bool,
) -> AnyElement {
    div()
        .flex()
        .items_center()
        .gap(px(24.))
        .px(px(16.))
        .py(px(14.))
        .when(divider, |el| el.border_b_1().border_color(t.border))
        .child(row_text(t, title, detail))
        .child(div().flex_none().child(control))
        .into_any_element()
}

/// iOS-style switch. The knob glides 150ms when `nonce` changes.
fn switch(t: Theme, id: &'static str, on: bool, nonce: usize) -> gpui::Stateful<gpui::Div> {
    const W: f32 = 36.;
    const KNOB: f32 = 16.;
    const PAD: f32 = 2.;
    let travel = W - KNOB - PAD * 2.;
    let knob = div()
        .size(px(KNOB))
        .rounded_full()
        .bg(if on {
            t.primary_text
        } else if t.bg == Theme::DARK.bg {
            Theme::DARK.text_secondary
        } else {
            Theme::LIGHT.surface
        })
        .shadow_sm();
    let knob = if nonce == 0 {
        knob.ml(px(if on { travel } else { 0. })).into_any_element()
    } else {
        knob.with_animation(
            (id, nonce),
            Animation::new(Duration::from_millis(150)).with_easing(ease_strong),
            move |el, d| {
                let x = if on { travel * d } else { travel * (1. - d) };
                el.ml(px(x))
            },
        )
        .into_any_element()
    };
    div()
        .id(id)
        .w(px(W))
        .h(px(KNOB + PAD * 2.))
        .p(px(PAD))
        .flex()
        .items_center()
        .rounded_full()
        .bg(if on { t.primary } else { t.border_strong })
        .cursor_pointer()
        .active(|s| s.opacity(0.85))
        .child(knob)
}

/// Small segmented control: the selected option sits on a raised surface.
pub(super) fn segmented<T: Copy + PartialEq + 'static>(
    t: Theme,
    id: &'static str,
    options: &[(T, &'static str)],
    current: T,
    on_pick: impl Fn(T, &mut gpui::Window, &mut gpui::App) + Clone + 'static,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap(px(2.))
        .p(px(2.))
        .rounded(px(RADIUS_SM_PX))
        .bg(t.track)
        .children(options.iter().enumerate().map(move |(i, &(value, label))| {
            let selected = value == current;
            let on_pick = on_pick.clone();
            div()
                .id((id, i))
                .h(px(28.))
                .px(px(12.))
                .flex()
                .items_center()
                .rounded(px(RADIUS_SM_PX - 2.))
                .text_size(px(13.))
                .font_weight(if selected { FontWeight::SEMIBOLD } else { FontWeight::MEDIUM })
                .text_color(if selected { t.text } else { t.text_secondary })
                .when(selected, |el| el.bg(t.surface).shadow_sm())
                .when(!selected, |el| el.hover(move |s| s.text_color(t.text)))
                .active(|s| s.opacity(0.8))
                .cursor_pointer()
                .child(label)
                .on_click(move |_, window, cx| on_pick(value, window, cx))
        }))
}

impl MainWindow {
    pub(super) fn render_settings(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx);
        let entity = cx.entity();
        let tabs: Vec<(SettingsTab, &'static str)> =
            SettingsTab::ALL.iter().map(|&tab| (tab, tab.label())).collect();
        let body = match self.settings_tab {
            SettingsTab::General => self.render_general(t, cx).into_any_element(),
            SettingsTab::Audio => self.render_audio(t, cx).into_any_element(),
            SettingsTab::Dictation => self.render_dictation(t, cx).into_any_element(),
            SettingsTab::Shortcuts => group(t, "Shortcuts", vec![self.render_hotkey(t).into_any_element()])
                .into_any_element(),
            SettingsTab::Appearance => self.render_appearance(t, cx).into_any_element(),
        };
        // Tab switch: plain 150ms cross-fade, no slide.
        let body = if self.tab_nonce == 0 {
            body
        } else {
            div()
                .w_full()
                .child(body)
                .with_animation(
                    ("settings-tab", self.tab_nonce),
                    Animation::new(Duration::from_millis(150)).with_easing(ease_strong),
                    |el, d| el.opacity(d),
                )
                .into_any_element()
        };
        div()
            .flex()
            .flex_col()
            .gap(px(24.))
            .w_full()
            .child(div().flex().child(segmented(
                t,
                "settings-tab",
                &tabs,
                self.settings_tab,
                move |tab, _, cx| {
                    entity.update(cx, |this, cx| this.set_settings_tab(tab, cx));
                },
            )))
            .child(div().flex().flex_col().gap(px(24.)).child(body))
    }

    fn render_general(&self, t: Theme, cx: &mut Context<Self>) -> impl IntoElement {
        let p = prefs(cx).clone();
        let entity = cx.entity();
        div()
            .flex()
            .flex_col()
            .gap(px(24.))
            .child(group(
                t,
                "Startup",
                vec![setting_row(
                    t,
                    "Launch at login",
                    "Open Hush when you sign in to Windows",
                    switch(t, "sw-login", self.launch_at_login, self.switch_nonce[0]).on_click(
                        cx.listener(|this, _, _, cx| {
                            this.launch_at_login = !this.launch_at_login;
                            autostart::set(this.launch_at_login);
                            this.launch_at_login = autostart::enabled();
                            this.switch_nonce[0] += 1;
                            cx.notify();
                        }),
                    ),
                    false,
                )],
            ))
            .child(group(
                t,
                "Recording",
                vec![
                    setting_row(
                        t,
                        "Sound cues",
                        "Play a soft tick when dictation starts and stops",
                        switch(t, "sw-sound", p.sound_cues, self.switch_nonce[1]).on_click(
                            cx.listener(|this, _, _, cx| {
                                this.switch_nonce[1] += 1;
                                update_prefs(cx, |p| p.sound_cues = !p.sound_cues);
                            }),
                        ),
                        true,
                    ),
                    setting_row(
                        t,
                        "Overlay position",
                        "Where the recording pill appears on screen",
                        segmented(
                            t,
                            "overlay-pos",
                            &[(OverlayPosition::Bottom, "Bottom"), (OverlayPosition::Top, "Top")],
                            p.overlay_position,
                            move |pos, _, cx| {
                                update_prefs(cx, |p| p.overlay_position = pos);
                                entity.update(cx, |_, cx| cx.notify());
                            },
                        ),
                        false,
                    ),
                ],
            ))
    }

    fn render_audio(&self, t: Theme, cx: &mut Context<Self>) -> impl IntoElement {
        let current = prefs(cx).mic.clone();
        // A saved device that is unplugged falls back to the default.
        let current = current.filter(|name| self.mic_devices.contains(name));
        let mut rows = Vec::new();
        let default_detail: SharedString = match &self.default_mic {
            Some(name) => format!("Follows Windows \u{00b7} {name}").into(),
            None => "Follows Windows".into(),
        };
        let count = self.mic_devices.len();
        rows.push(self.mic_row(t, 0, "System default".into(), default_detail, None, current.is_none(), count > 0, cx));
        for (i, name) in self.mic_devices.iter().enumerate() {
            rows.push(self.mic_row(
                t,
                i + 1,
                SharedString::from(name.clone()),
                "".into(),
                Some(name.clone()),
                current.as_deref() == Some(name.as_str()),
                i + 1 < count,
                cx,
            ));
        }
        let level = self.mic_level.clamp(0., 1.);
        div()
            .flex()
            .flex_col()
            .gap(px(24.))
            .child(group(t, "Microphone", rows))
            .child(group(
                t,
                "Input level",
                vec![setting_row(
                    t,
                    "Test your microphone",
                    if self.mic_test.is_some() { "Speak and watch the meter move" } else { "No microphone available" },
                    div()
                        .w(px(180.))
                        .h(px(6.))
                        .rounded_full()
                        .bg(t.track)
                        .overflow_hidden()
                        .child(div().h_full().w(px(180. * level)).rounded_full().bg(t.primary)),
                    false,
                )],
            ))
    }

    #[allow(clippy::too_many_arguments)]
    fn mic_row(
        &self,
        t: Theme,
        index: usize,
        name: SharedString,
        detail: SharedString,
        device: Option<String>,
        selected: bool,
        divider: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        div()
            .id(("mic", index))
            .flex()
            .items_center()
            .gap(px(24.))
            .px(px(16.))
            .py(px(12.))
            .when(index == 0, |el| el.rounded_t(px(RADIUS_PX - 1.)))
            .when(!divider, |el| el.rounded_b(px(RADIUS_PX - 1.)))
            .when(divider, |el| el.border_b_1().border_color(t.border))
            .when(!selected, |el| el.hover(move |s| s.bg(t.surface_hover)))
            .active(move |s| s.bg(t.surface_active))
            .cursor_pointer()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w(px(0.))
                    .gap(px(2.))
                    .child(
                        div()
                            .text_size(px(14.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(t.text)
                            .truncate()
                            .child(name),
                    )
                    .when(!detail.is_empty(), |el| {
                        el.child(div().text_size(px(12.)).text_color(t.text_muted).truncate().child(detail))
                    }),
            )
            .child(selection_dot(t, selected, self.mic_nonce))
            .on_click(cx.listener(move |this, _, _, cx| {
                if prefs(cx).mic != device {
                    this.mic_nonce += 1;
                    let device = device.clone();
                    update_prefs(cx, move |p| p.mic = device);
                    // Reopen the test meter on the new device.
                    this.mic_test = None;
                }
            }))
            .into_any_element()
    }

    fn render_dictation(&self, t: Theme, cx: &mut Context<Self>) -> impl IntoElement {
        let style = match prefs(cx).style {
            crate::prefs::Style::Formal => "Formal",
            crate::prefs::Style::Casual => "Casual",
            crate::prefs::Style::Verbatim => "Verbatim",
        };
        div()
            .flex()
            .flex_col()
            .gap(px(24.))
            .child(group(t, "Model", vec![self.render_model_list(t, cx).into_any_element()]))
            .child(group(
                t,
                "Output",
                vec![setting_row(
                    t,
                    "Writing style",
                    format!("{style} \u{00b7} how your words read once typed"),
                    div()
                        .id("open-style")
                        .h(px(30.))
                        .px(px(12.))
                        .flex()
                        .items_center()
                        .gap(px(4.))
                        .rounded(px(RADIUS_SM_PX))
                        .border_1()
                        .border_color(t.border_strong)
                        .text_size(px(13.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(t.text)
                        .hover(move |s| s.bg(t.surface_hover))
                        .active(|s| s.opacity(0.8))
                        .cursor_pointer()
                        .child("Change")
                        .on_click(cx.listener(|this, _, _, cx| this.set_page(Page::Style, cx))),
                    false,
                )],
            ))
    }

    fn render_appearance(&self, t: Theme, cx: &mut Context<Self>) -> impl IntoElement {
        let current = theme_mode(cx);
        let body = div()
            .flex()
            .flex_col()
            .gap(px(16.))
            .px(px(16.))
            .py(px(16.))
            .child(row_text(t, "Window theme", "Light or dark; the recording pill is always dark"))
            .child(
                div()
                    .flex()
                    .justify_center()
                    .gap(px(24.))
                    .child(self.theme_preview(t, current, "Light", ThemeMode::Light, Theme::LIGHT, cx))
                    .child(self.theme_preview(t, current, "Dark", ThemeMode::Dark, Theme::DARK, cx)),
            );
        group(t, "Theme", vec![body.into_any_element()])
    }

    fn theme_preview(
        &self,
        t: Theme,
        current: ThemeMode,
        label: &'static str,
        mode: ThemeMode,
        preview: Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selected = current == mode;
        div()
            .id(label)
            .flex()
            .flex_col()
            .items_center()
            .gap(px(10.))
            .cursor_pointer()
            .active(|s| s.opacity(0.92))
            .child(
                div()
                    .p(px(3.))
                    .rounded(px(12.))
                    .border_2()
                    .border_color(if selected { t.primary } else { t.surface })
                    .child(home_miniature(preview)),
            )
            .child(
                div()
                    .text_size(px(13.))
                    .font_weight(if selected { FontWeight::SEMIBOLD } else { FontWeight::NORMAL })
                    .text_color(if selected { t.text } else { t.text_secondary })
                    .child(label),
            )
            .on_click(cx.listener(move |_, _, _, cx| {
                set_theme_mode(mode, cx);
                cx.notify();
            }))
    }

    fn render_model_list(&self, t: Theme, cx: &mut Context<Self>) -> impl IntoElement {
        let last = self.models.len().saturating_sub(1);
        let mut rows = Vec::new();
        for (index, model) in self.models.iter().enumerate() {
            rows.push(self.model_row(t, index, model, index < last, cx).into_any_element());
        }
        div().flex().flex_col().children(rows)
    }

    fn model_row(
        &self,
        t: Theme,
        index: usize,
        model: &ModelOption,
        show_divider: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selected = self.active_model == index;
        div()
            .id(model.id)
            .flex()
            .items_center()
            .justify_between()
            .px(px(16.))
            .py(px(14.))
            // GPUI doesn't clip child fills to the card radius: round the ends.
            .when(index == 0, |el| el.rounded_t(px(RADIUS_PX - 1.)))
            .when(!show_divider, |el| el.rounded_b(px(RADIUS_PX - 1.)))
            .when(show_divider, |el| el.border_b_1().border_color(t.border))
            .when(selected, |el| el.bg(t.surface_selected))
            .when(!selected, |el| el.hover(move |s| s.bg(t.surface_hover)))
            .active(move |s| s.bg(t.surface_active))
            .cursor_pointer()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .child(
                        div()
                            .text_size(px(14.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(t.text)
                            .child(model.name.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(t.text_muted)
                            .child(model.detail.clone()),
                    ),
            )
            .child(selection_dot(t, selected, self.dot_nonce))
            .on_click(cx.listener(move |this, _, _, cx| this.select_model(index, cx)))
    }

    fn render_hotkey(&self, t: Theme) -> impl IntoElement {
        let hold: Vec<SharedString> =
            self.hotkey.split(" + ").map(|k| SharedString::from(k.to_string())).collect();
        div()
            .flex()
            .flex_col()
            .child(shortcut_row(t, "Hold to dictate", "Hold to record, release to stop", hold, true))
            .child(shortcut_row(
                t,
                "Toggle dictation",
                "Press to lock recording on, press again to stop",
                vec!["Ctrl".into(), "Win".into(), "Alt".into()],
                false,
            ))
    }
}

/// One shortcut row: title/detail on the left, key caps on the right.
fn shortcut_row(
    t: Theme,
    title: &'static str,
    detail: &'static str,
    keys: Vec<SharedString>,
    divider: bool,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_between()
        .px(px(16.))
        .py(px(14.))
        .when(divider, |el| el.border_b_1().border_color(t.border))
        .child(row_text(t, title, detail))
        .child(div().flex().items_center().gap(px(4.)).children(keys.into_iter().map(move |k| {
            div()
                .min_w(px(26.))
                .flex()
                .justify_center()
                .px(px(8.))
                .py(px(4.))
                .rounded(px(RADIUS_SM_PX))
                .bg(t.track)
                .border_1()
                .border_color(t.border_strong)
                .text_size(px(12.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(t.text)
                .child(k)
        })))
}

/// Tiny Home tab, painted in a fixed palette so Light and Dark can sit side by side.
fn home_miniature(t: Theme) -> impl IntoElement {
    div()
        .w(px(228.))
        .h(px(148.))
        .rounded(px(10.))
        .overflow_hidden()
        .bg(t.bg)
        .border_1()
        .border_color(t.border_strong)
        .shadow_sm()
        .flex()
        .flex_col()
        .child(
            div()
                .h(px(16.))
                .flex_none()
                .flex()
                .items_center()
                .px(px(8.))
                .child(
                    div()
                        .text_size(px(7.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(t.text)
                        .child("Hush"),
                ),
        )
        .child(
            div()
                .flex()
                .flex_1()
                .min_h(px(0.))
                .child(mini_sidebar(t))
                .child(mini_home_panel(t)),
        )
}

fn mini_sidebar(t: Theme) -> impl IntoElement {
    div()
        .w(px(52.))
        .h_full()
        .flex_none()
        .flex()
        .flex_col()
        .justify_between()
        .px(px(4.))
        .py(px(4.))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(2.))
                .child(mini_nav(t, "icons/home.svg", true))
                .child(mini_nav(t, "icons/book.svg", false))
                .child(mini_nav(t, "icons/keyboard.svg", false)),
        )
        .child(mini_nav(t, "icons/settings.svg", false))
}

fn mini_nav(t: Theme, icon: &'static str, selected: bool) -> impl IntoElement {
    let fg = if selected { t.text } else { t.text_secondary };
    div()
        .h(px(14.))
        .px(px(4.))
        .rounded(px(3.))
        .flex()
        .items_center()
        .when(selected, |el| el.bg(t.nav_selected))
        .child(svg().path(icon).size(px(8.)).text_color(fg))
}

fn mini_home_panel(t: Theme) -> impl IntoElement {
    div()
        .flex_1()
        .h_full()
        .min_w(px(0.))
        .pr(px(4.))
        .pb(px(4.))
        .child(
            div()
                .size_full()
                .rounded(px(6.))
                .bg(t.panel)
                .overflow_hidden()
                .px(px(10.))
                .pt(px(8.))
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(
                            div()
                                .text_size(px(9.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(t.text)
                                .child("Welcome back!"),
                        )
                        .child(
                            div()
                                .text_size(px(6.))
                                .text_color(t.text_muted)
                                .child("Hold Ctrl + Space to dictate"),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .gap(px(4.))
                        .child(mini_stat(t, "142"))
                        .child(mini_stat(t, "3,184"))
                        .child(mini_stat(t, "4")),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(3.))
                        .child(
                            div()
                                .text_size(px(6.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(t.text_secondary)
                                .child("History"),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .rounded(px(6.))
                                .bg(t.surface)
                                .border_1()
                                .border_color(t.border)
                                .overflow_hidden()
                                .child(mini_history_row(t, "Schedule a design review\u{2026}", true))
                                .child(mini_history_row(t, "Remind me to send the notes\u{2026}", false)),
                        ),
                ),
        )
}

fn mini_stat(t: Theme, value: &'static str) -> impl IntoElement {
    div()
        .flex_1()
        .rounded(px(6.))
        .bg(t.surface)
        .border_1()
        .border_color(t.border)
        .px(px(6.))
        .py(px(5.))
        .child(
            div()
                .text_size(px(9.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(t.text)
                .child(value),
        )
}

fn mini_history_row(t: Theme, text: &'static str, divider: bool) -> impl IntoElement {
    div()
        .px(px(6.))
        .py(px(5.))
        .when(divider, |el| el.border_b_1().border_color(t.border))
        .child(div().text_size(px(6.)).text_color(t.text).child(text))
}
