//! Speech-to-text: whisper.cpp models (download, storage) and inference.
//! Everything here is blocking; callers run it off the UI thread.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperState};

use crate::theme::app_dir;

#[derive(Clone, Copy)]
pub(crate) struct ModelInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub detail: &'static str,
    pub file: &'static str,
    pub size_mb: u32,
    pub english_only: bool,
}

pub(crate) const MODELS: [ModelInfo; 3] = [
    ModelInfo {
        id: "fast",
        name: "Fast",
        detail: "Base \u{00b7} English only \u{00b7} near-instant",
        file: "ggml-base.en-q5_1.bin",
        size_mb: 57,
        english_only: true,
    },
    ModelInfo {
        id: "balanced",
        name: "Balanced",
        detail: "Small \u{00b7} multilingual \u{00b7} recommended",
        file: "ggml-small-q5_1.bin",
        size_mb: 181,
        english_only: false,
    },
    ModelInfo {
        id: "best",
        name: "Best",
        detail: "Large v3 Turbo \u{00b7} most accurate, slower",
        file: "ggml-large-v3-turbo-q5_0.bin",
        size_mb: 547,
        english_only: false,
    },
];

pub(crate) const CUSTOM_ID: &str = "custom";

pub(crate) fn model_info(id: &str) -> Option<ModelInfo> {
    MODELS.iter().copied().find(|m| m.id == id)
}

pub(crate) fn models_dir() -> Option<PathBuf> {
    Some(app_dir()?.join("models"))
}

pub(crate) fn model_path(m: &ModelInfo) -> Option<PathBuf> {
    Some(models_dir()?.join(m.file))
}

pub(crate) fn is_installed(m: &ModelInfo) -> bool {
    model_path(m).is_some_and(|p| p.is_file())
}

/// The file to transcribe with for a model id, if it exists on disk.
pub(crate) fn resolve(id: &str, custom: Option<&str>) -> Option<PathBuf> {
    let path = if id == CUSTOM_ID {
        PathBuf::from(custom?)
    } else {
        model_path(&model_info(id)?)?
    };
    path.is_file().then_some(path)
}

/// `.en` files only speak English, so the language choice is moot.
pub(crate) fn is_english_only(path: &Path) -> bool {
    path.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.contains(".en"))
}

// ---------------------------------------------------------------- download

pub(crate) struct Download {
    pub downloaded: AtomicU64,
    pub total: AtomicU64,
    pub cancel: AtomicBool,
    /// 0 running, 1 done, 2 failed, 3 cancelled.
    pub status: AtomicU8,
}

impl Download {
    pub(crate) fn progress(&self) -> f32 {
        let total = self.total.load(Ordering::Relaxed);
        if total == 0 {
            return 0.;
        }
        self.downloaded.load(Ordering::Relaxed) as f32 / total as f32
    }

    pub(crate) fn running(&self) -> bool {
        self.status.load(Ordering::Relaxed) == 0
    }
}

/// Start downloading `m` on a background thread. Writes `<file>.part`, then
/// renames it into place, so a half-finished file is never picked up.
pub(crate) fn start_download(m: ModelInfo) -> Arc<Download> {
    let dl = Arc::new(Download {
        downloaded: AtomicU64::new(0),
        total: AtomicU64::new(m.size_mb as u64 * 1_048_576),
        cancel: AtomicBool::new(false),
        status: AtomicU8::new(0),
    });
    let state = dl.clone();
    std::thread::spawn(move || {
        let result = download_blocking(&m, &state);
        let status = match result {
            Ok(()) => 1,
            Err(_) if state.cancel.load(Ordering::Relaxed) => 3,
            Err(err) => {
                eprintln!("hush: download {} failed: {err}", m.file);
                2
            }
        };
        state.status.store(status, Ordering::Relaxed);
    });
    dl
}

fn download_blocking(m: &ModelInfo, dl: &Download) -> Result<(), String> {
    let dir = models_dir().ok_or("no app data dir")?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let url = format!("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/{}", m.file);
    let resp = ureq::get(&url).call().map_err(|e| e.to_string())?;
    if let Some(len) = resp.header("Content-Length").and_then(|v| v.parse::<u64>().ok()) {
        dl.total.store(len, Ordering::Relaxed);
    }
    let part = dir.join(format!("{}.part", m.file));
    let mut out = std::fs::File::create(&part).map_err(|e| e.to_string())?;
    let mut reader = resp.into_reader();
    let mut buf = vec![0u8; 256 * 1024];
    loop {
        if dl.cancel.load(Ordering::Relaxed) {
            drop(out);
            let _ = std::fs::remove_file(&part);
            return Err("cancelled".into());
        }
        let n = reader.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        out.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        dl.downloaded.fetch_add(n as u64, Ordering::Relaxed);
    }
    out.flush().map_err(|e| e.to_string())?;
    drop(out);
    std::fs::rename(&part, dir.join(m.file)).map_err(|e| e.to_string())?;
    Ok(())
}

pub(crate) fn delete_model(m: &ModelInfo) {
    if let Some(path) = model_path(m) {
        unload_if(&path);
        let _ = std::fs::remove_file(path);
    }
}

// --------------------------------------------------------------- inference

/// Language codes the settings offer ("auto" = detect).
pub(crate) const LANGS: &[&str] = &["auto", "en", "es", "fr", "de", "it", "pt", "nl"];

/// Loaded model, kept warm between dictations.
static CONTEXT: Mutex<Option<(PathBuf, Arc<WhisperContext>)>> = Mutex::new(None);

fn context(path: &Path) -> Result<Arc<WhisperContext>, String> {
    let mut slot = CONTEXT.lock().unwrap();
    if let Some((p, ctx)) = slot.as_ref()
        && p == path
    {
        return Ok(ctx.clone());
    }
    whisper_rs::install_logging_hooks();
    let ctx = WhisperContext::new_with_params(
        path.to_str().ok_or("bad model path")?,
        WhisperContextParameters::default(),
    )
    .map_err(|e| format!("load model: {e}"))?;
    let ctx = Arc::new(ctx);
    *slot = Some((path.to_path_buf(), ctx.clone()));
    Ok(ctx)
}

/// Decoder state from the last dictation, reused by the next one.
static STATE: Mutex<Option<(PathBuf, WhisperState)>> = Mutex::new(None);

fn unload_if(path: &Path) {
    {
        let mut st = STATE.lock().unwrap();
        if st.as_ref().is_some_and(|(p, _)| p == path) {
            *st = None;
        }
    }
    let mut slot = CONTEXT.lock().unwrap();
    if slot.as_ref().is_some_and(|(p, _)| p == path) {
        *slot = None;
    }
}

/// Load the model ahead of the first dictation (no-op if already warm).
pub(crate) fn preload(path: &Path) {
    if context(path).is_ok() && STATE.lock().unwrap().is_none() {
        // Warm the state with half a second of silence so the first
        // auto-detect dictation is fast too.
        let _ = transcribe(path, &[0.0; 8_000], "en", "");
    }
}

pub(crate) struct Transcript {
    pub text: String,
    pub load_ms: u128,
    pub infer_ms: u128,
}

/// Transcribe 16 kHz mono samples. `language` is "auto" or an ISO code;
/// `prompt` biases spelling (dictionary words).
pub(crate) fn transcribe(
    path: &Path,
    samples: &[f32],
    language: &str,
    prompt: &str,
) -> Result<Transcript, String> {
    let t0 = Instant::now();
    let ctx = context(path)?;
    let load_ms = t0.elapsed().as_millis();
    let t1 = Instant::now();
    // Reuse the warm state: it keeps the last clip's encoder size, which
    // keeps language auto-detect on the short window (see below).
    let mut slot = STATE.lock().unwrap();
    let (mut state, warm) = match slot.take() {
        Some((p, st)) if p == path => (st, true),
        _ => (ctx.create_state().map_err(|e| e.to_string())?, false),
    };
    let cores = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    // Linux: leave a core for the UI / X server (2-CPU VMs froze otherwise).
    let cores = if cfg!(target_os = "linux") { cores.saturating_sub(1).max(1) } else { cores };
    let threads = cores.min(8) as i32;
    let lang = if is_english_only(path) { "en" } else { language };
    // Whisper always encodes a 30 s window; dictation clips are short, so
    // shrink the encoder context to the clip (50 frames/s plus headroom).
    // Big speedup on CPU. HUSH_FULL_CTX=1 turns it off for comparison.
    let audio_ctx = if std::env::var_os("HUSH_FULL_CTX").is_none() {
        let secs = samples.len() as f32 / 16_000.0;
        let frames = ((secs * 50.0) as i32 + 128 + 63) / 64 * 64;
        (frames < 1500).then_some(frames)
    } else {
        None
    };
    let base = |lang: &'static str| {
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_n_threads(threads);
        params.set_translate(false);
        params.set_no_timestamps(true);
        params.set_single_segment(true);
        params.set_suppress_blank(true);
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_language(Some(lang));
        if let Some(frames) = audio_ctx {
            params.set_audio_ctx(frames);
        }
        params
    };
    // Language auto-detect runs before whisper.cpp applies audio_ctx, so on
    // a fresh state it would encode a full 30 s window (4-5x slower). A tiny
    // primer pass sets the state's context size first; warm states skip it.
    if lang == "auto" && audio_ctx.is_some() && !warm {
        let mut primer = base("en");
        primer.set_max_tokens(1);
        primer.set_temperature_inc(0.0);
        let n = samples.len().min(16_000);
        state.full(primer, &samples[..n]).map_err(|e| e.to_string())?;
    }
    let lang: &'static str = LANGS.iter().copied().find(|l| *l == lang).unwrap_or("auto");
    let mut params = base(lang);
    if !prompt.is_empty() {
        params.set_initial_prompt(prompt);
    }
    state.full(params, samples).map_err(|e| e.to_string())?;
    let n = state.full_n_segments();
    let mut text = String::new();
    for i in 0..n {
        if let Some(seg) = state.get_segment(i) {
            text.push_str(&seg.to_str_lossy().map_err(|e| e.to_string())?);
        }
    }
    *slot = Some((path.to_path_buf(), state));
    Ok(Transcript { text: text.trim().to_string(), load_ms, infer_ms: t1.elapsed().as_millis() })
}
