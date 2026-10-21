//! Microphone level meter for the recording pill.
//!
//! Opens the default input device with cpal, computes the RMS of each audio
//! buffer on the audio thread and publishes it as f32 bits in an AtomicU32.
//! The stream is released when the `MicMeter` is dropped (overlay hidden).

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream};

pub(crate) struct MicMeter {
    level: Arc<AtomicU32>,
    _stream: Stream,
}

impl MicMeter {
    /// `None` when there is no input device or it can't be opened
    /// (e.g. mic permission denied); the pill then shows a static mark.
    pub(crate) fn start() -> Option<Self> {
        let device = cpal::default_host().default_input_device()?;
        let config = device.default_input_config().ok()?;
        let level = Arc::new(AtomicU32::new(0f32.to_bits()));
        let format = config.sample_format();
        let config = config.into();
        let on_err = |err| eprintln!("hush: mic stream error: {err}");

        let stream = match format {
            SampleFormat::F32 => {
                let level = level.clone();
                device.build_input_stream(
                    &config,
                    move |data: &[f32], _| publish(&level, data.iter().copied()),
                    on_err,
                    None,
                )
            }
            SampleFormat::I16 => {
                let level = level.clone();
                device.build_input_stream(
                    &config,
                    move |data: &[i16], _| {
                        publish(&level, data.iter().map(|s| *s as f32 / i16::MAX as f32))
                    },
                    on_err,
                    None,
                )
            }
            SampleFormat::U16 => {
                let level = level.clone();
                device.build_input_stream(
                    &config,
                    move |data: &[u16], _| {
                        publish(&level, data.iter().map(|s| (*s as f32 - 32768.) / 32768.))
                    },
                    on_err,
                    None,
                )
            }
            _ => return None,
        }
        .ok()?;
        stream.play().ok()?;
        Some(Self { level, _stream: stream })
    }

    /// Latest buffer RMS, roughly 0..1.
    pub(crate) fn level(&self) -> f32 {
        f32::from_bits(self.level.load(Ordering::Relaxed))
    }
}

fn publish(level: &AtomicU32, samples: impl Iterator<Item = f32>) {
    let (mut sum, mut n) = (0f32, 0usize);
    for s in samples {
        sum += s * s;
        n += 1;
    }
    if n > 0 {
        level.store((sum / n as f32).sqrt().to_bits(), Ordering::Relaxed);
    }
}
