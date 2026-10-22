//! Dictation history: one JSON object per line in
//! `%APPDATA%\hush-voice\history.jsonl`, newest last on disk.

use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

use gpui::{App, Global};
use serde::{Deserialize, Serialize};

use crate::models::TranscriptRow;
use crate::prefs::KeepHistory;
use crate::theme::app_dir;

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Entry {
    pub text: String,
    /// Unix seconds.
    pub ts: i64,
    pub words: usize,
    /// Recording length in seconds.
    pub seconds: f32,
    #[serde(default)]
    pub model: String,
}

/// Newest first, in memory.
pub(crate) struct History(pub Vec<Entry>);

impl Global for History {}

fn path() -> Option<std::path::PathBuf> {
    Some(app_dir()?.join("history.jsonl"))
}

pub(crate) fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

fn load() -> Vec<Entry> {
    let raw = path().and_then(|p| std::fs::read_to_string(p).ok()).unwrap_or_default();
    let mut rows: Vec<Entry> = raw.lines().filter_map(|l| serde_json::from_str(l).ok()).collect();
    rows.reverse();
    rows
}

fn rewrite(rows: &[Entry]) {
    let Some(path) = path() else { return };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let body: String = rows
        .iter()
        .rev()
        .filter_map(|e| serde_json::to_string(e).ok())
        .map(|l| l + "\n")
        .collect();
    let _ = std::fs::write(path, body);
}

pub(crate) fn init(cx: &mut App, keep: KeepHistory) {
    let mut rows = load();
    if prune(&mut rows, keep) {
        rewrite(&rows);
    }
    cx.set_global(History(rows));
}

/// Drop entries older than the retention window. True if any were removed.
fn prune(rows: &mut Vec<Entry>, keep: KeepHistory) -> bool {
    let Some(days) = keep.days() else { return false };
    let cutoff = now() - days * 86_400;
    let before = rows.len();
    rows.retain(|e| e.ts >= cutoff);
    rows.len() != before
}

pub(crate) fn apply_retention(cx: &mut App, keep: KeepHistory) {
    let rows = &mut cx.global_mut::<History>().0;
    if prune(rows, keep) {
        rewrite(rows);
    }
    cx.refresh_windows();
}

pub(crate) fn add(cx: &mut App, text: &str, seconds: f32, model: &str) {
    let entry = Entry {
        text: text.to_string(),
        ts: now(),
        words: text.split_whitespace().count(),
        seconds,
        model: model.to_string(),
    };
    if let Some(path) = path() {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path)
            && let Ok(line) = serde_json::to_string(&entry)
        {
            let _ = writeln!(f, "{line}");
        }
    }
    cx.global_mut::<History>().0.insert(0, entry);
    cx.refresh_windows();
}

pub(crate) fn clear(cx: &mut App) {
    cx.global_mut::<History>().0.clear();
    if let Some(path) = path() {
        let _ = std::fs::remove_file(path);
    }
    cx.refresh_windows();
}

/// Rows for the Home list and stats.
pub(crate) fn rows(cx: &App) -> Vec<TranscriptRow> {
    cx.global::<History>()
        .0
        .iter()
        .map(|e| TranscriptRow::from_entry(&e.text, &e.model, e.seconds, e.ts.div_euclid(86_400)))
        .collect()
}
