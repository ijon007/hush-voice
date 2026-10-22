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

    pub(crate) fn words(&self) -> usize {
        self.text.split_whitespace().count()
    }}

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
    /// Typing time (at 40 wpm) minus speaking time, over timed rows.
    pub minutes_saved: u32,
    /// Words per day over the last 7 days, oldest first (today last).
    pub week: [usize; 7],
}

/// WPM = words across rows with a known duration / their total minutes.
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

    // ponytail: 40 wpm is the commonly cited average typing speed.
    let minutes_saved = (timed_words as f32 / 40.0 - seconds / 60.0).max(0.0).round() as u32;

    let mut week = [0usize; 7];
    let now = today();
    for r in rows {
        if let Some(ago) = r.day.map(|d| now - d).filter(|a| (0..7).contains(a)) {
            week[6 - ago as usize] += r.words();
        }
    }

    Stats { wpm, total_words, minutes_saved, week }
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

/// Hesitation sounds every style drops.
const FILLERS: &[&str] = &["um", "umm", "uh", "uhh", "er", "erm", "hmm", "mm"];

/// Rewrite raw transcript text in the chosen output style. Every style
/// drops filler words.
/// - Formal: punctuation kept, sentences and "I" capitalized, closing period added.
/// - Casual: sentences and "I" capitalized; only . ? ! at sentence ends survive.
/// - Lowercase: like Casual, but everything lowercase (even "i").
pub(crate) fn apply_style(text: &str, style: crate::prefs::Style) -> String {
    use crate::prefs::Style;
    // Drop filler words (and the comma that often trails them).
    let mut words: Vec<String> = Vec::new();
    for raw in text.split_whitespace() {
        let core = raw.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase();
        if FILLERS.contains(&core.as_str()) {
            // "should, uh, ship" -> "should ship"; keep a sentence end glued to it.
            if let Some(prev) = words.last_mut() {
                let mut fixed = prev.trim_end_matches(',').to_string();
                if let Some(end) = raw.chars().last().filter(|c| matches!(c, '.' | '?' | '!')) {
                    if !fixed.ends_with(['.', '?', '!']) {
                        fixed.push(end);
                    }
                }
                *prev = fixed;
            }
            continue;
        }
        let word = if style == Style::Formal { raw.to_string() } else { plain_word(raw) };
        if !word.is_empty() {
            words.push(word);
        }
    }
    let mut out = String::new();
    let mut sentence_start = true;
    for w in &words {
        if !out.is_empty() {
            out.push(' ');
        }
        if style == Style::Lowercase {
            out.push_str(&w.to_lowercase());
        } else {
            let lower = w.to_lowercase();
            let is_i = lower == "i" || lower.starts_with("i'");
            // Skip leading quotes/brackets so `"so` capitalizes the s.
            let lead = w.find(|c: char| c.is_alphanumeric()).unwrap_or(0);
            let (head, body) = w.split_at(lead);
            let mut chars = body.chars();
            out.push_str(head);
            if let Some(first) = chars.next() {
                if sentence_start || is_i {
                    out.extend(first.to_uppercase());
                } else {
                    out.push(first);
                }
            }
            out.push_str(chars.as_str());
        }
        sentence_start = w.ends_with(['.', '?', '!']);
    }
    let out = out.trim_end_matches([',', ';', ':']).to_string();
    match style {
        Style::Formal if !out.is_empty() && !out.ends_with(['.', '?', '!']) => format!("{out}."),
        _ => out,
    }
}

/// Casual/Lowercase word: no commas, colons, dashes or quotes; apostrophes
/// inside words and in-word symbols (emails, decimals, hyphenated words)
/// stay, and a trailing . ? ! survives as the sentence end.
fn plain_word(raw: &str) -> String {
    let chars: Vec<char> = raw.chars().collect();
    let first = chars.iter().position(|c| c.is_alphanumeric());
    let Some(first) = first else {
        // Bare punctuation ("-", "...") carries no words; drop it.
        return String::new();
    };
    let last = chars.iter().rposition(|c| c.is_alphanumeric()).unwrap_or(first);
    let mut out = String::new();
    for &c in &chars[first..=last] {
        // Quotes are always dropped; ASCII apostrophes stay (contractions).
        if matches!(c, '"' | '\u{201c}' | '\u{201d}' | '\u{2014}' | '\u{2013}' | ',' | ';' | ':') {
            continue;
        }
        out.push(if c == '\u{2019}' { '\'' } else { c });
    }
    if let Some(end) = chars[last + 1..].iter().rev().find(|c| matches!(c, '.' | '?' | '!')) {
        out.push(*end);
    }
    out
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
        let raw = "um so i think we should, uh, ship the build on Friday: the new one. what do you think?";
        assert_eq!(
            apply_style(raw, Style::Formal),
            "So I think we should ship the build on Friday: the new one. What do you think?"
        );
        assert_eq!(
            apply_style(raw, Style::Casual),
            "So I think we should ship the build on Friday the new one. What do you think?"
        );
        assert_eq!(
            apply_style(raw, Style::Lowercase),
            "so i think we should ship the build on friday the new one. what do you think?"
        );
    }

    #[test]
    fn formal_keeps_commas_and_adds_period() {
        use crate::prefs::Style;
        assert_eq!(
            apply_style("well, uh, i'm not sure, honestly", Style::Formal),
            "Well I'm not sure, honestly."
        );
    }

    #[test]
    fn casual_strips_mid_sentence_punctuation() {
        use crate::prefs::Style;
        assert_eq!(
            apply_style("ok - \"great\"; let's go, i'm in! really?", Style::Casual),
            "Ok great let's go I'm in! Really?"
        );
        // No closing period is invented.
        assert_eq!(apply_style("um see you soon", Style::Casual), "See you soon");
    }

    #[test]
    fn lowercase_keeps_contractions_and_in_word_symbols() {
        use crate::prefs::Style;
        assert_eq!(
            apply_style("I Can't Mail ada@example.com, it's 3.5 MB.", Style::Lowercase),
            "i can't mail ada@example.com it's 3.5 mb."
        );
    }

    #[test]
    fn every_style_drops_fillers() {
        use crate::prefs::Style;
        for style in [Style::Formal, Style::Casual, Style::Lowercase] {
            let out = apply_style("uh, hmm, we ship, um, today.", style).to_lowercase();
            assert!(!out.contains("uh") && !out.contains("hmm") && !out.contains("um"), "{out}");
        }
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
