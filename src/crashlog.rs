//! Panic reports written to %APPDATA%\hush-voice\logs (one file per crash).

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) fn logs_dir() -> Option<PathBuf> {
    Some(crate::theme::app_dir()?.join("logs"))
}

/// Linux: append a timestamped line to logs/hush.log (key events, so a
/// freeze can be diagnosed afterwards). No-op elsewhere.
pub(crate) fn debug(msg: &str) {
    #[cfg(target_os = "linux")]
    {
        use std::io::Write;
        let Some(dir) = logs_dir() else { return };
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("hush.log");
        // Keep it small: start over past 1 MB.
        if std::fs::metadata(&path).map(|m| m.len() > 1 << 20).unwrap_or(false) {
            let _ = std::fs::remove_file(&path);
        }
        let ms = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
            let _ = writeln!(f, "{ms} {msg}");
        }
    }
    #[cfg(not(target_os = "linux"))]
    let _ = msg;
}

/// Install the panic hook. Keeps the default hook (stderr) as well.
pub(crate) fn install() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = write_report(info);
        default(info);
    }));
}

fn write_report(info: &std::panic::PanicHookInfo<'_>) -> Option<()> {
    let dir = logs_dir()?;
    std::fs::create_dir_all(&dir).ok()?;
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
    let thread = std::thread::current();
    let body = format!(
        "Hush {version} crash\nunix time: {secs}\nos: {os}\nthread: {thread}\n\n{info}\n\nbacktrace:\n{bt}\n",
        version = env!("CARGO_PKG_VERSION"),
        os = std::env::consts::OS,
        thread = thread.name().unwrap_or("unnamed"),
        bt = std::backtrace::Backtrace::force_capture(),
    );
    std::fs::write(dir.join(format!("crash-{secs}.log")), body).ok()
}

/// Open the logs folder in Explorer (created if missing).
pub(crate) fn open_folder() {
    if let Some(dir) = logs_dir() {
        let _ = std::fs::create_dir_all(&dir);
        #[cfg(windows)]
        let _ = std::process::Command::new("explorer").arg(dir).spawn();
        #[cfg(target_os = "linux")]
        crate::platform::linux::open(&dir);
    }
}
