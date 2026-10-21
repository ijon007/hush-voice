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
    pub duration: SharedString,
}

pub(crate) fn sample_history() -> Vec<TranscriptRow> {
    vec![
        TranscriptRow {
            text: "Schedule a design review for Thursday afternoon.".into(),
            model: "Whisper Local".into(),
            duration: "2.4s".into(),
        },
        TranscriptRow {
            text: "Remind me to send the hush-voice build notes.".into(),
            model: "Cloud STT".into(),
            duration: "1.8s".into(),
        },
        TranscriptRow {
            text: "Draft a calm reply declining the meeting.".into(),
            model: "Whisper Local".into(),
            duration: "3.1s".into(),
        },
        TranscriptRow {
            text: "Open the settings panel and pick a model.".into(),
            model: "Whisper Large".into(),
            duration: "2.0s".into(),
        },
        TranscriptRow {
            text: "Capture action items from the standup recording.".into(),
            model: "Whisper Local".into(),
            duration: "4.2s".into(),
        },
        TranscriptRow {
            text: "Rewrite the onboarding copy to feel quieter.".into(),
            model: "Cloud STT".into(),
            duration: "2.7s".into(),
        },
        TranscriptRow {
            text: "Pin the overlay near the cursor on the next pass.".into(),
            model: "Whisper Local".into(),
            duration: "3.4s".into(),
        },
        TranscriptRow {
            text: "Check whether SendInput needs a focus fallback.".into(),
            model: "Whisper Large".into(),
            duration: "2.1s".into(),
        },
        TranscriptRow {
            text: "Ship a tray icon before wiring the SQLite history store.".into(),
            model: "Cloud STT".into(),
            duration: "3.9s".into(),
        },
        TranscriptRow {
            text: "Keep the recording pill critically damped — no bounce.".into(),
            model: "Whisper Local".into(),
            duration: "2.6s".into(),
        },
    ]
}
