//! Update check against the latest GitHub release. Runs once on launch and
//! from Settings; any network or parse failure is silent.

use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use gpui::{App, Global, Timer};

pub(crate) const VERSION: &str = env!("CARGO_PKG_VERSION");
const LATEST: &str = "https://api.github.com/repos/ijon007/hush-voice/releases/latest";

#[derive(Clone)]
pub(crate) struct Release {
    pub version: String,
    /// Release page (fallback when the installer can't be fetched).
    pub url: String,
    /// Direct download of the Hush-Setup-*.exe asset, when attached.
    pub installer: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Status {
    Idle,
    Checking,
    UpToDate,
    Failed,
}

pub(crate) struct Updates {
    pub status: Status,
    /// Set only when the latest release is newer than this build.
    pub available: Option<Release>,
    /// Installer download in flight.
    pub downloading: bool,
    /// Download progress in permille (0 when the size is unknown).
    pub progress: Arc<AtomicU32>,
}

impl Global for Updates {}

pub(crate) fn init(cx: &mut App) {
    cx.set_global(Updates {
        status: Status::Idle,
        available: None,
        downloading: false,
        progress: Arc::new(AtomicU32::new(0)),
    });
    // Debug builds: HUSH_FAKE_UPDATE=0.2.0 pretends that release exists.
    #[cfg(debug_assertions)]
    if let Ok(version) = std::env::var("HUSH_FAKE_UPDATE") {
        cx.global_mut::<Updates>().available = Some(Release {
            version: version.trim_start_matches('v').to_string(),
            url: "https://github.com/ijon007/hush-voice/releases/latest".into(),
            installer: None,
        });
        return;
    }
    check(cx);
}

pub(crate) fn updates(cx: &App) -> &Updates {
    cx.global::<Updates>()
}

pub(crate) fn check(cx: &mut App) {
    if cx.global::<Updates>().status == Status::Checking {
        return;
    }
    cx.global_mut::<Updates>().status = Status::Checking;
    cx.refresh_windows();
    let task = cx.background_executor().spawn(async { fetch_latest() });
    cx.spawn(async move |cx| {
        let latest = task.await;
        let _ = cx.update(|cx| {
            let u = cx.global_mut::<Updates>();
            match latest {
                Some(r) if is_newer(&r.version, VERSION) => {
                    u.available = Some(r);
                    u.status = Status::Idle;
                }
                Some(_) => {
                    u.available = None;
                    u.status = Status::UpToDate;
                }
                None => u.status = Status::Failed,
            }
            cx.refresh_windows();
        });
    })
    .detach();
}

fn fetch_latest() -> Option<Release> {
    let body = ureq::get(LATEST)
        .set("User-Agent", concat!("hush-voice/", env!("CARGO_PKG_VERSION")))
        .set("Accept", "application/vnd.github+json")
        .timeout(Duration::from_secs(8))
        .call()
        .ok()?
        .into_string()
        .ok()?;
    let json: serde_json::Value = serde_json::from_str(&body).ok()?;
    let tag = json["tag_name"].as_str()?;
    let url = json["html_url"].as_str()?;
    let installer = json["assets"].as_array().and_then(|assets| {
        assets.iter().find_map(|a| {
            let name = a["name"].as_str()?;
            (name.starts_with("Hush-Setup-") && name.ends_with(".exe"))
                .then(|| a["browser_download_url"].as_str().map(str::to_string))
                .flatten()
        })
    });
    Some(Release { version: tag.trim_start_matches('v').to_string(), url: url.to_string(), installer })
}

/// Download the installer to %TEMP%, run it and quit. Falls back to the
/// release page if anything goes wrong.
pub(crate) fn install(cx: &mut App) {
    let u = cx.global::<Updates>();
    let Some(release) = u.available.clone() else { return };
    if u.downloading {
        return;
    }
    let Some(asset) = release.installer.clone() else {
        cx.open_url(&release.url);
        return;
    };
    let progress = u.progress.clone();
    progress.store(0, Ordering::Relaxed);
    cx.global_mut::<Updates>().downloading = true;
    cx.refresh_windows();
    let task = cx.background_executor().spawn({
        let progress = progress.clone();
        async move { download(&asset, &progress) }
    });
    // Repaint the progress while the download runs.
    cx.spawn(async move |cx| {
        loop {
            Timer::after(Duration::from_millis(150)).await;
            let running = cx
                .update(|cx| {
                    cx.refresh_windows();
                    cx.global::<Updates>().downloading
                })
                .unwrap_or(false);
            if !running {
                break;
            }
        }
    })
    .detach();
    cx.spawn(async move |cx| {
        let file = task.await;
        let _ = cx.update(|cx| {
            let launched = file.is_some_and(|path| std::process::Command::new(path).spawn().is_ok());
            if launched {
                cx.quit();
                return;
            }
            cx.global_mut::<Updates>().downloading = false;
            cx.open_url(&release.url);
            cx.refresh_windows();
        });
    })
    .detach();
}

fn download(url: &str, progress: &AtomicU32) -> Option<PathBuf> {
    let name = url.rsplit('/').next().filter(|n| n.ends_with(".exe")).unwrap_or("Hush-Setup.exe");
    let path = std::env::temp_dir().join(name);
    let part = path.with_extension("part");
    let resp = ureq::get(url)
        .set("User-Agent", concat!("hush-voice/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(600))
        .call()
        .ok()?;
    let total: u64 = resp.header("Content-Length").and_then(|v| v.parse().ok()).unwrap_or(0);
    let mut reader = resp.into_reader();
    let mut file = std::fs::File::create(&part).ok()?;
    let mut buf = vec![0u8; 64 * 1024];
    let mut done: u64 = 0;
    loop {
        let n = match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(_) => {
                let _ = std::fs::remove_file(&part);
                return None;
            }
        };
        if file.write_all(&buf[..n]).is_err() {
            let _ = std::fs::remove_file(&part);
            return None;
        }
        done += n as u64;
        if total > 0 {
            progress.store((done * 1000 / total).min(1000) as u32, Ordering::Relaxed);
        }
    }
    drop(file);
    if total > 0 && done != total {
        let _ = std::fs::remove_file(&part);
        return None;
    }
    let _ = std::fs::remove_file(&path);
    std::fs::rename(&part, &path).ok()?;
    Some(path)
}

/// "1.10.0" > "1.9.3"; pre-release suffixes ("-beta") are ignored.
pub(crate) fn is_newer(candidate: &str, current: &str) -> bool {
    fn parts(v: &str) -> Vec<u64> {
        let core = v.trim().trim_start_matches('v').split(['-', '+']).next().unwrap_or("");
        core.split('.').map(|p| p.parse().unwrap_or(0)).collect()
    }
    let (mut a, mut b) = (parts(candidate), parts(current));
    let n = a.len().max(b.len());
    a.resize(n, 0);
    b.resize(n, 0);
    a > b
}

#[cfg(test)]
mod tests {
    use super::is_newer;

    #[test]
    fn compares_versions_numerically() {
        assert!(is_newer("0.2.0", "0.1.0"));
        assert!(is_newer("v1.10.0", "1.9.3"));
        assert!(is_newer("1.0", "0.9.9"));
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(!is_newer("0.1.0-beta", "0.1.0"));
        assert!(!is_newer("0.0.9", "0.1.0"));
    }
}
