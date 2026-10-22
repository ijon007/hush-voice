#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod app_icon;
mod crashlog;
mod assets;
mod chord;
mod hotkey;
mod main_window;
mod models;
mod engine;
mod history;
mod paste;
mod overlay;
mod pipeline;
mod prefs;
mod sounds;
mod text_field;
mod theme;
mod tray;
mod update;
mod widgets;

use gpui::{
    App, Application, Bounds, TitlebarOptions, WindowBounds, WindowOptions, prelude::*, px, size,
};

use hotkey::register_hotkey;
use main_window::MainWindow;
use overlay::{PERSIST_OVERLAY, show_overlay};

fn main() {
    crashlog::install();
    // Debug tools (no window):
    //   hush-voice --transcribe <file.wav> [model-id-or-path] [language]
    //   hush-voice --download <fast|balanced|best>
    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 3 && args[1] == "--transcribe" {
        std::process::exit(cli_transcribe(&args[2], args.get(3).map(String::as_str), args.get(4).cloned()));
    }
    if args.len() >= 3 && args[1] == "--download" {
        std::process::exit(cli_download(&args[2]));
    }
    Application::new().with_assets(assets::Assets).run(|cx: &mut App| {
        theme::init(cx);
        prefs::init(cx);
        let keep = prefs::prefs(cx).keep_history;
        history::init(cx, keep);
        register_hotkey(cx);
        update::init(cx);
        tray::init(cx);

        let (w, h) = window_size(cx);
        let bounds = Bounds::centered(None, size(px(w), px(h)), cx);
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
                window_min_size: Some(size(px(MIN_W.min(w)), px(MIN_H.min(h)))),
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

/// Designed size of the main window (logical px).
const WINDOW_W: f32 = 1042.;
const WINDOW_H: f32 = 756.;
/// Smallest the layout still works at.
const MIN_W: f32 = 760.;
const MIN_H: f32 = 520.;

/// Designed size, shrunk to fit the monitor's work area (taskbar excluded).
/// At 125% on a 1366x768 laptop the designed height doesn't fit.
fn window_size(cx: &App) -> (f32, f32) {
    let Some((aw, ah)) = work_area(cx) else { return (WINDOW_W, WINDOW_H) };
    // Leave a margin so the window doesn't touch the screen edges.
    let w = WINDOW_W.min(aw - 32.).max(MIN_W.min(aw));
    let h = WINDOW_H.min(ah - 32.).max(MIN_H.min(ah));
    (w, h)
}

/// Primary monitor work area in logical px.
#[cfg(windows)]
fn work_area(cx: &App) -> Option<(f32, f32)> {
    use windows::Win32::Foundation::RECT;
    use windows::Win32::UI::WindowsAndMessaging::{
        SPI_GETWORKAREA, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SystemParametersInfoW,
    };
    let mut r = RECT::default();
    unsafe {
        SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            Some(&mut r as *mut RECT as *mut _),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
        .ok()?;
    }
    // Work area is in physical px; the display bounds tell us the scale.
    let display = cx.primary_display()?;
    let logical_w = f32::from(display.bounds().size.width);
    let phys_w = unsafe {
        windows::Win32::UI::WindowsAndMessaging::GetSystemMetrics(
            windows::Win32::UI::WindowsAndMessaging::SM_CXSCREEN,
        )
    } as f32;
    let scale = if logical_w > 0. && phys_w > 0. { phys_w / logical_w } else { 1. };
    Some(((r.right - r.left) as f32 / scale, (r.bottom - r.top) as f32 / scale))
}

#[cfg(not(windows))]
fn work_area(cx: &App) -> Option<(f32, f32)> {
    let s = cx.primary_display()?.bounds().size;
    Some((f32::from(s.width), f32::from(s.height) - 48.))
}

fn cli_transcribe(wav: &str, model: Option<&str>, lang: Option<String>) -> i32 {
    let prefs = prefs::load_standalone();
    let id = model.unwrap_or(&prefs.model);
    let path = engine::resolve(id, prefs.custom_model.as_deref())
        .or_else(|| Some(std::path::PathBuf::from(id)).filter(|p| p.is_file()));
    let Some(path) = path else {
        eprintln!("no model '{id}' on disk");
        return 2;
    };
    let mut reader = match hound::WavReader::open(wav) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("open {wav}: {e}");
            return 2;
        }
    };
    let spec = reader.spec();
    let raw: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader.samples::<f32>().filter_map(Result::ok).collect(),
        hound::SampleFormat::Int => {
            let max = (1i64 << (spec.bits_per_sample - 1)) as f32;
            reader.samples::<i32>().filter_map(Result::ok).map(|s| s as f32 / max).collect()
        }
    };
    let ch = spec.channels.max(1) as usize;
    let mono: Vec<f32> = raw.chunks(ch).map(|c| c.iter().sum::<f32>() / ch as f32).collect();
    let settings = pipeline::Settings {
        style: prefs.style,
        shortcuts: main_window::load_shortcuts(),
        words: main_window::load_words(),
        // Optional third argument overrides the saved language.
        language: lang.unwrap_or_else(|| prefs.language.clone()),
    };
    let t0 = std::time::Instant::now();
    match pipeline::run(&path, &mono, spec.sample_rate, &settings) {
        Ok(Some(out)) => {
            println!("model: {}", path.display());
            println!("audio: {:.2}s", out.seconds);
            println!("load: {} ms, inference: {} ms, total: {} ms", out.load_ms, out.infer_ms, t0.elapsed().as_millis());
            println!("raw: {}", out.raw);
            println!("text: {}", out.text);
            0
        }
        Ok(None) => {
            println!("(nothing to transcribe)");
            0
        }
        Err(e) => {
            eprintln!("error: {e}");
            1
        }
    }
}

fn cli_download(id: &str) -> i32 {
    let Some(m) = engine::model_info(id) else {
        eprintln!("unknown model '{id}'");
        return 2;
    };
    let dl = engine::start_download(m);
    let t0 = std::time::Instant::now();
    while dl.running() {
        std::thread::sleep(std::time::Duration::from_millis(500));
        eprint!("\r{:.0}%   ", dl.progress() * 100.);
    }
    let ok = dl.status.load(std::sync::atomic::Ordering::Relaxed) == 1;
    eprintln!("\n{} in {:.1}s", if ok { "done" } else { "failed" }, t0.elapsed().as_secs_f32());
    if ok { 0 } else { 1 }
}
