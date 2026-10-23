//! Soft start/stop cues: two short sine notes, synthesized once into WAV
//! buffers and played asynchronously so they never block the UI.

use std::sync::OnceLock;

#[derive(Clone, Copy)]
pub(crate) enum Cue {
    Start,
    Stop,
}

const RATE: u32 = 44_100;

fn wav(notes: &[(f32, f32)]) -> Vec<u8> {
    let mut pcm: Vec<i16> = Vec::new();
    for &(freq, secs) in notes {
        let n = (RATE as f32 * secs) as usize;
        for i in 0..n {
            let t = i as f32 / RATE as f32;
            // 5ms attack, exponential decay: a soft "tick", not a beep.
            let attack = (t / 0.005).min(1.);
            let env = attack * (-t / (secs * 0.35)).exp();
            let s = (t * freq * std::f32::consts::TAU).sin() * env * 0.18;
            pcm.push((s * i16::MAX as f32) as i16);
        }
    }
    let data_len = (pcm.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&RATE.to_le_bytes());
    out.extend_from_slice(&(RATE * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in pcm {
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}

fn buffer(cue: Cue) -> &'static [u8] {
    static START: OnceLock<Vec<u8>> = OnceLock::new();
    static STOP: OnceLock<Vec<u8>> = OnceLock::new();
    match cue {
        // Rising fifth on start, falling on stop.
        Cue::Start => START.get_or_init(|| wav(&[(660., 0.06), (990., 0.09)])),
        Cue::Stop => STOP.get_or_init(|| wav(&[(990., 0.06), (660., 0.09)])),
    }
}

#[cfg(windows)]
pub(crate) fn play(cue: Cue) {
    use windows::Win32::Media::Audio::{PlaySoundW, SND_ASYNC, SND_MEMORY, SND_NODEFAULT};
    let buf = buffer(cue);
    unsafe {
        let _ = PlaySoundW(
            windows::core::PCWSTR(buf.as_ptr() as *const u16),
            None,
            SND_MEMORY | SND_ASYNC | SND_NODEFAULT,
        );
    }
}

#[cfg(target_os = "linux")]
pub(crate) fn play(cue: Cue) {
    let name = match cue {
        Cue::Start => "start",
        Cue::Stop => "stop",
    };
    crate::platform::linux::play_wav(name, buffer(cue));
}

#[cfg(not(any(windows, target_os = "linux")))]
pub(crate) fn play(_cue: Cue) {
    let _ = buffer;
}
