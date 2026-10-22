//! Microphone level meter for the recording pill.
//!
//! Opens the default input device with cpal, computes the RMS of each audio
//! buffer on the audio thread and publishes it as f32 bits in an AtomicU32.
//! The stream is released when the `MicMeter` is dropped (overlay hidden).

use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU32, Ordering};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream};

pub(crate) struct MicMeter {
    level: Arc<AtomicU32>,
    /// Mono samples captured since start (for transcription).
    buffer: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
    _stream: Stream,
}

impl MicMeter {
    /// `None` when there is no input device or it can't be opened
    /// (e.g. mic permission denied); the pill then shows a static mark.
    /// Opens `name` if it is connected, else the Windows default input.
    pub(crate) fn start(name: Option<&str>) -> Option<Self> {
        let host = cpal::default_host();
        let named = name.and_then(|want| {
            host.input_devices().ok()?.find(|d| d.name().ok().as_deref() == Some(want))
        });
        let device = named.or_else(|| host.default_input_device())?;
        let config = device.default_input_config().ok()?;
        let level = Arc::new(AtomicU32::new(0f32.to_bits()));
        let format = config.sample_format();
        let sample_rate = config.sample_rate().0;
        let channels = config.channels().max(1) as usize;
        let buffer: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
        let config = config.into();
        let on_err = |err| eprintln!("hush: mic stream error: {err}");

        let stream = match format {
            SampleFormat::F32 => {
                let level = level.clone();
                let buf = buffer.clone();
                device.build_input_stream(
                    &config,
                    move |data: &[f32], _| publish(&level, &buf, channels, data.iter().copied()),
                    on_err,
                    None,
                )
            }
            SampleFormat::I16 => {
                let level = level.clone();
                let buf = buffer.clone();
                device.build_input_stream(
                    &config,
                    move |data: &[i16], _| {
                        publish(&level, &buf, channels, data.iter().map(|s| *s as f32 / i16::MAX as f32))
                    },
                    on_err,
                    None,
                )
            }
            SampleFormat::U16 => {
                let level = level.clone();
                let buf = buffer.clone();
                device.build_input_stream(
                    &config,
                    move |data: &[u16], _| {
                        publish(&level, &buf, channels, data.iter().map(|s| (*s as f32 - 32768.) / 32768.))
                    },
                    on_err,
                    None,
                )
            }
            _ => return None,
        }
        .ok()?;
        stream.play().ok()?;
        Some(Self { level, buffer, sample_rate, _stream: stream })
    }

    /// Everything captured so far (mono) and its sample rate.
    pub(crate) fn take_samples(&self) -> (Vec<f32>, u32) {
        let samples = std::mem::take(&mut *self.buffer.lock().unwrap());
        (samples, self.sample_rate)
    }

    /// Latest buffer RMS, roughly 0..1.
    pub(crate) fn level(&self) -> f32 {
        f32::from_bits(self.level.load(Ordering::Relaxed))
    }
}

/// Names of the connected input devices, in system order.
pub(crate) fn input_device_names() -> Vec<String> {
    cpal::default_host()
        .input_devices()
        .map(|it| it.filter_map(|d| d.name().ok()).collect())
        .unwrap_or_default()
}

fn publish(
    level: &AtomicU32,
    buffer: &Mutex<Vec<f32>>,
    channels: usize,
    samples: impl Iterator<Item = f32>,
) {
    let (mut sum, mut n) = (0f32, 0usize);
    let mut mono = Vec::new();
    let (mut acc, mut k) = (0f32, 0usize);
    for s in samples {
        sum += s * s;
        n += 1;
        acc += s;
        k += 1;
        if k == channels {
            mono.push(acc / channels as f32);
            acc = 0.;
            k = 0;
        }
    }
    if n > 0 {
        level.store((sum / n as f32).sqrt().to_bits(), Ordering::Relaxed);
    }
    // Cap at 10 minutes so a forgotten lock can't eat all memory.
    if let Ok(mut buf) = buffer.lock()
        && buf.len() < 48_000 * 600
    {
        buf.extend_from_slice(&mono);
    }
}
