use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

use gpui::SharedString;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Page {
    Home,
    Dictionary,
    Shortcuts,
    Style,
    Settings,
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
    fn on_day(
        text: impl Into<SharedString>,
        model: impl Into<SharedString>,
        seconds: f32,
        day: Option<i64>,
    ) -> Self {
        Self { text: text.into(), model: model.into(), seconds, day }
    }

    pub(crate) fn from_entry(text: &str, model: &str, seconds: f32, day: i64) -> Self {
        Self::on_day(text.to_string(), model.to_string(), seconds, Some(day))
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

/// A spoken word (or phrase) rewritten before the transcript is committed.
/// Saying `trigger` inserts `expansion` instead.
#[derive(Clone)]
pub(crate) struct Shortcut {
    pub trigger: SharedString,
    pub expansion: SharedString,
}

impl Shortcut {
    pub(crate) fn new(trigger: impl Into<SharedString>, expansion: impl Into<SharedString>) -> Self {
        Self { trigger: trigger.into(), expansion: expansion.into() }
    }
}

/// Replace shortcut triggers in `text` with their expansions.
/// Match is case-insensitive and only on a whole word or phrase, so "email"
/// rewrites "send email." and leaves "emailing" alone. Longer triggers win.
pub(crate) fn apply_shortcuts(text: &str, shortcuts: &[Shortcut]) -> String {
    let mut rules: Vec<&Shortcut> = shortcuts.iter().filter(|s| !s.trigger.is_empty()).collect();
    rules.sort_by_key(|s| std::cmp::Reverse(s.trigger.chars().count()));

    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        let at_boundary = i == 0 || !is_shortcut_word(chars[i - 1]);
        if at_boundary
            && let Some(rule) = rules.iter().find(|rule| trigger_matches(&chars, i, &rule.trigger))
        {
            out.push_str(&rule.expansion);
            i += rule.trigger.chars().count();
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

fn is_shortcut_word(c: char) -> bool {
    c.is_alphanumeric() || c == '\'' || c == '-' || c == '_'
}

fn trigger_matches(chars: &[char], start: usize, trigger: &str) -> bool {
    let mut n = 0;
    for (k, ch) in trigger.chars().enumerate() {
        let Some(got) = chars.get(start + k) else { return false };
        if !got.eq_ignore_ascii_case(&ch) {
            return false;
        }
        n = k + 1;
    }
    if n == 0 {
        return false;
    }
    let end = start + n;
    end == chars.len() || !is_shortcut_word(chars[end])
}

/// Hesitation sounds dropped by Formal and Casual.
const FILLERS: &[&str] = &["um", "umm", "uh", "uhh", "er", "erm", "hmm", "mm"];

/// Rewrite raw transcript text in the chosen output style.
/// - Formal: fillers removed, sentences capitalized, closing period added.
/// - Casual: fillers removed, lowercase sentence starts (keeps "I"), no final period.
/// - Verbatim: unchanged.
pub(crate) fn apply_style(text: &str, style: crate::prefs::Style) -> String {
    use crate::prefs::Style;
    if style == Style::Verbatim {
        return text.trim().to_string();
    }
    // Drop filler words (and the comma that often trails them).
    let mut words: Vec<String> = Vec::new();
    for raw in text.split_whitespace() {
        let core = raw.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase();
        if FILLERS.contains(&core.as_str()) {
            // "should, uh, ship" -> "should ship"; keep a sentence end glued to it.
            if let Some(prev) = words.last_mut() {
                let mut fixed = prev.trim_end_matches(',').to_string();
                if let Some(end) = raw.chars().last().filter(|c| matches!(c, '.' | '?' | '!')) {
                    fixed.push(end);
                }
                *prev = fixed;
            }
            continue;
        }
        words.push(raw.to_string());
    }
    let mut out = String::new();
    let mut sentence_start = true;
    for w in &words {
        if !out.is_empty() {
            out.push(' ');
        }
        let mut chars = w.chars();
        let first = chars.next().unwrap_or_default();
        let rest: String = chars.collect();
        let is_i = w == "i" || w.starts_with("i'") || w == "I" || w.starts_with("I'");
        let first = match style {
            Style::Formal if sentence_start || is_i => first.to_uppercase().collect::<String>(),
            Style::Casual if sentence_start && !is_i => first.to_lowercase().collect(),
            Style::Casual if is_i => first.to_uppercase().collect(),
            _ => first.to_string(),
        };
        out.push_str(&first);
        out.push_str(&rest);
        sentence_start = matches!(w.chars().last(), Some('.' | '?' | '!'));
    }
    let out = out.trim_end_matches(',').to_string();
    match style {
        Style::Formal if !out.is_empty() && !out.ends_with(['.', '?', '!']) => format!("{out}."),
        Style::Casual => out.strip_suffix('.').map(str::to_string).unwrap_or(out),
        _ => out,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_shortcut_replaces_the_spoken_word() {
        let shortcuts = [Shortcut::new("email", "ada@example.com")];
        assert_eq!(
            apply_shortcuts("Send it to email today.", &shortcuts),
            "Send it to ada@example.com today."
        );
        assert_eq!(apply_shortcuts("Email me", &shortcuts), "ada@example.com me");
        assert_eq!(apply_shortcuts("emailing the list", &shortcuts), "emailing the list");
    }

    #[test]
    fn styles_rewrite_the_same_sentence() {
        use crate::prefs::Style;
        let raw = "um so i think we should, uh, ship the build on Friday. what do you think?";
        assert_eq!(
            apply_style(raw, Style::Formal),
            "So I think we should ship the build on Friday. What do you think?"
        );
        assert_eq!(
            apply_style(raw, Style::Casual),
            "so I think we should ship the build on Friday. what do you think?"
        );
        assert_eq!(apply_style(raw, Style::Verbatim), raw);
    }

    #[test]
    fn longer_phrase_wins() {
        let shortcuts = [
            Shortcut::new("email", "ada@example.com"),
            Shortcut::new("work email", "ada@studio.com"),
        ];
        assert_eq!(
            apply_shortcuts("my work email is on file", &shortcuts),
            "my ada@studio.com is on file"
        );
    }
}
