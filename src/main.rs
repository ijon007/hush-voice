#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod hotkey;
mod main_window;
mod models;
mod overlay;
mod theme;
mod widgets;

use gpui::{
    App, Application, Bounds, TitlebarOptions, WindowBounds, WindowOptions, prelude::*, px, size,
};

use hotkey::register_hotkey;
use main_window::MainWindow;
use overlay::{PERSIST_OVERLAY, show_overlay};

fn main() {
    Application::new().run(|cx: &mut App| {
        theme::init(cx);
        register_hotkey(cx);

        let bounds = Bounds::centered(None, size(px(1280.), px(720.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("Hush".into()),
                    appears_transparent: true,
                    ..Default::default()
                }),
                is_resizable: false,
                is_minimizable: true,
                window_min_size: Some(size(px(720.), px(520.))),
                ..Default::default()
            },
            |window, cx| cx.new(|cx| MainWindow::new(window, cx)),
        )
        .unwrap();

        if PERSIST_OVERLAY {
            show_overlay(cx);
        }

        cx.activate(true);
    });
}
