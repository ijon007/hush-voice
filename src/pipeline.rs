//! Recorded audio -> finished text: resample, trim, transcribe, clean up.

use std::path::Path;

use crate::engine;
use crate::models::{Shortcut, apply_shortcuts, apply_style};
use crate::prefs::Style;

pub(crate) const RATE: u32 = 16_000;

/// Linear resample of mono audio to 16 kHz (fine for speech).
pub(crate) fn resample(samples: &[f32], from: u32) -> Vec<f32> {
    if from == RATE || samples.is_empty() {
        return samples.to_vec();
    }
    let ratio = from as f64 / RATE as f64;
    let n = (samples.len() as f64 / ratio) as usize;
    (0..n)
        .map(|i| {
            let x = i as f64 * ratio;
            let j = x as usize;
            let f = (x - j as f64) as f32;
            let a = samples[j];
            let b = *samples.get(j + 1).unwrap_or(&a);
            a + (b - a) * f
        })
        .collect()
}

/// Trim leading/trailing silence (20ms frames, 200ms padding kept).
/// `None` when the clip is too short (< 0.3s of sound) or silent.
pub(crate) fn prepare(samples: &[f32], rate: u32) -> Option<Vec<f32>> {
    let audio = resample(samples, rate);
    const FRAME: usize = 320;
    let rms: Vec<f32> = audio
        .chunks(FRAME)
        .map(|c| (c.iter().map(|s| s * s).sum::<f32>() / c.len() as f32).sqrt())
        .collect();
    let peak = rms.iter().copied().fold(0f32, f32::max);
    if peak < 0.004 {
        return None;
    }
    let threshold = (peak * 0.08).max(0.003);
    let first = rms.iter().position(|&r| r > threshold)?;
    let last = rms.iter().rposition(|&r| r > threshold)?;
    let pad = 10; // frames = 200ms
    let start = first.saturating_sub(pad) * FRAME;
    let end = ((last + 1 + pad) * FRAME).min(audio.len());
    let voiced = (last + 1 - first) * FRAME;
    if voiced < (RATE as usize * 3) / 10 {
        return None;
    }
    let mut clip = audio[start..end].to_vec();
    // whisper.cpp wants at least 1s; pad short clips with silence.
    if clip.len() < RATE as usize + RATE as usize / 10 {
        clip.resize(RATE as usize + RATE as usize / 10, 0.);
    }
    Some(clip)
}

/// Whisper's non-speech markers like "[BLANK_AUDIO]" or "(music)".
fn strip_markers(text: &str) -> String {
    let mut out = String::new();
    let mut depth = 0;
    for c in text.chars() {
        match c {
            '[' | '(' => depth += 1,
            ']' | ')' if depth > 0 => depth -= 1,
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(crate) fn postprocess(raw: &str, style: Style, shortcuts: &[Shortcut]) -> String {
    let text = strip_markers(raw);
    let text = apply_shortcuts(&text, shortcuts);
    apply_style(&text, style)
}

/// Dictionary words as a whisper prompt, so it prefers those spellings.
pub(crate) fn vocabulary_prompt(words: &[String]) -> String {
    if words.is_empty() {
        String::new()
    } else {
        format!("Vocabulary: {}.", words.join(", "))
    }
}

pub(crate) struct Settings {
    pub style: Style,
    pub shortcuts: Vec<Shortcut>,
    pub words: Vec<String>,
    pub language: String,
}

pub(crate) struct Outcome {
    pub text: String,
    pub raw: String,
    pub seconds: f32,
    pub load_ms: u128,
    pub infer_ms: u128,
}

/// The whole pipeline. `Ok(None)` = nothing worth typing (silence, too short).
pub(crate) fn run(
    model: &Path,
    samples: &[f32],
    rate: u32,
    s: &Settings,
) -> Result<Option<Outcome>, String> {
    let seconds = samples.len() as f32 / rate.max(1) as f32;
    let Some(clip) = prepare(samples, rate) else { return Ok(None) };
    let t = engine::transcribe(model, &clip, &s.language, &vocabulary_prompt(&s.words))?;
    let text = postprocess(&t.text, s.style, &s.shortcuts);
    if text.is_empty() {
        return Ok(None);
    }
    Ok(Some(Outcome { text, raw: t.text, seconds, load_ms: t.load_ms, infer_ms: t.infer_ms }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silence_and_short_clips_are_skipped() {
        assert!(prepare(&vec![0.0; 48_000], 48_000).is_none());
        let mut blip = vec![0.0; 48_000];
        for s in blip.iter_mut().take(4_800) {
            *s = 0.3;
        }
        assert!(prepare(&blip, 48_000).is_none(), "0.1s is too short");
    }

    #[test]
    fn markers_are_removed() {
        assert_eq!(strip_markers("[BLANK_AUDIO] hello (music) world"), "hello world");
    }

    #[test]
    fn resample_halves_length() {
        assert_eq!(resample(&vec![0.0; 32_000], 32_000).len(), 16_000);
    }
}
