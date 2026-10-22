//! Update check against the latest GitHub release. Runs once on launch and
//! from Settings; any network or parse failure is silent.

use std::time::Duration;

use gpui::{App, Global};

pub(crate) const VERSION: &str = env!("CARGO_PKG_VERSION");
const LATEST: &str = "https://api.github.com/repos/ijon007/hush-voice/releases/latest";

#[derive(Clone)]
pub(crate) struct Release {
    pub version: String,
    pub url: String,
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
}

impl Global for Updates {}

pub(crate) fn init(cx: &mut App) {
    cx.set_global(Updates { status: Status::Idle, available: None });
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
    Some(Release { version: tag.trim_start_matches('v').to_string(), url: url.to_string() })
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
