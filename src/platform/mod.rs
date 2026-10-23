//! OS system layer that isn't already behind `#[cfg(windows)]` in the feature
//! modules. Windows code stays where it was (hotkey.rs, paste.rs, tray.rs,
//! overlay.rs, prefs.rs, update.rs); the Linux side lives here and is called
//! from the `#[cfg(not(windows))]` branches. See docs/LINUX.md.

#[cfg(target_os = "linux")]
pub(crate) mod linux;
