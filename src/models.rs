use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

use gpui::SharedString;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Page {
    Home,
    Dictionary,
    Shortcuts,
    Settings,
}

#[derive(Clone)]
pub(crate) struct ModelOption {
    pub id: &'static str,
    pub name: SharedString,
    pub detail: SharedString,
}

#[derive(Clone)]
pub(crate) struct TranscriptRow {
    pub text: SharedString,
    pub model: SharedString,
    /// Recording length in seconds (0 when unknown).
    pub seconds: f32,
    /// Day the dictation happened, as days since the Unix epoch.
    /// `None` for entries recorded before timestamps existed.
    pub day: Option<i64>,
}

impl TranscriptRow {
    /// A new dictation captured now (stamped with today's date).
    pub(crate) fn new(text: impl Into<SharedString>, model: impl Into<SharedString>, seconds: f32) -> Self {
        Self::on_day(text, model, seconds, Some(today()))
    }

    fn on_day(
        text: impl Into<SharedString>,
        model: impl Into<SharedString>,
        seconds: f32,
        day: Option<i64>,
    ) -> Self {
        Self { text: text.into(), model: model.into(), seconds, day }
    }

    pub(crate) fn duration_label(&self) -> SharedString {
        format!("{:.1}s", self.seconds).into()
    }

    fn words(&self) -> usize {
        self.text.split_whitespace().count()
    }
}

/// Today as days since the Unix epoch (UTC).
pub(crate) fn today() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| (d.as_secs() / 86_400) as i64)
        .unwrap_or(0)
}

pub(crate) struct Stats {
    pub wpm: u32,
    pub total_words: usize,
    pub streak_days: u32,
}

/// WPM = words across rows with a known duration / their total minutes.
/// Streak = consecutive days with a dictation, ending today (or yesterday,
/// so the streak isn't "lost" before you've dictated today). Undated rows
/// count toward words and WPM but not the streak.
pub(crate) fn compute_stats(rows: &[TranscriptRow]) -> Stats {
    let total_words = rows.iter().map(TranscriptRow::words).sum();

    let (timed_words, seconds) = rows
        .iter()
        .filter(|r| r.seconds > 0.0)
        .fold((0usize, 0f32), |(w, s), r| (w + r.words(), s + r.seconds));
    let wpm = if seconds > 0.0 {
        (timed_words as f32 / seconds * 60.0).round() as u32
    } else {
        0
    };

    let days: HashSet<i64> = rows.iter().filter_map(|r| r.day).collect();
    let mut day = today();
    if !days.contains(&day) {
        day -= 1;
    }
    let mut streak_days = 0;
    while days.contains(&day) {
        streak_days += 1;
        day -= 1;
    }

    Stats { wpm, total_words, streak_days }
}

pub(crate) fn sample_history() -> Vec<TranscriptRow> {
    let d = today();
    let mut rows = vec![
        TranscriptRow::new("Schedule a design review for Thursday afternoon.", "Whisper Local", 2.4),
        TranscriptRow::new("Remind me to send the hush-voice build notes.", "Cloud STT", 1.8),
    ];
    let older: [(&str, &str, f32, Option<i64>); 8] = [
        ("Draft a calm reply declining the meeting.", "Whisper Local", 3.1, Some(d - 1)),
        ("Open the settings panel and pick a model.", "Whisper Large", 2.0, Some(d - 1)),
        ("Capture action items from the standup recording.", "Whisper Local", 4.2, Some(d - 2)),
        ("Rewrite the onboarding copy to feel quieter.", "Cloud STT", 2.7, Some(d - 3)),
        ("Pin the overlay near the cursor on the next pass.", "Whisper Local", 3.4, Some(d - 5)),
        ("Check whether SendInput needs a focus fallback.", "Whisper Large", 2.1, Some(d - 6)),
        ("Ship a tray icon before wiring the SQLite history store.", "Cloud STT", 3.9, None),
        ("Keep the recording pill critically damped \u{2014} no bounce.", "Whisper Local", 2.6, None),
    ];
    rows.extend(older.into_iter().map(|(t, m, s, day)| TranscriptRow::on_day(t, m, s, day)));
    rows
}
