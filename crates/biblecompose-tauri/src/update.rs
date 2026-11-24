//! Checking for a newer release, and installing it, when asked (ADR-008).
//!
//! The second request this application can make, after fetching a font
//! (FONT-006), and fenced the same way: **a person asks for it by clicking a
//! button**, it goes to one place, and nothing identifying goes with it. There
//! is no check at start-up and none on a timer. A publisher setting a Bible
//! on a machine that is never online should never see this module do
//! anything, and one who is online should not find their copy reporting to
//! anybody because it was opened.
//!
//! # What is and is not allowed out
//!
//! [`ENDPOINT`] is the only address asked. It is a fixed file on this
//! project's GitHub releases, with no query string and no version or platform
//! in the path, so the request says nothing about who is asking. It is set
//! here in code rather than in `tauri.conf.json` so that the test pinning it
//! and the place it is used are the same constant.
//!
//! What it answers names a download, and that address is checked against
//! [`DOWNLOADS`] before anything is fetched: a manifest that pointed anywhere
//! else is refused, not followed. The bytes are then verified against the
//! release's signing key by the updater itself, and an installer whose
//! signature does not match is never run.
//!
//! # A copy that cannot update
//!
//! The public key comes from the release workflow, which writes it into the
//! configuration only when the repository has a signing key to go with it. A
//! development build, or a release cut without one, has an empty key, and
//! [`check`] says so rather than asking the network a question whose answer
//! it could not trust.

use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Runtime};
use tauri_plugin_updater::{Update, UpdaterExt};

/// The one address asked whether there is a newer release.
///
/// `releases/latest` is GitHub's name for the newest *published* release, so
/// a draft — which is what the release workflow makes — is never offered. A
/// person publishes it, and that click is what ships an update.
pub const ENDPOINT: &str =
    "https://github.com/samueldotj/bible-compose/releases/latest/download/latest.json";

/// Where every download the manifest names has to live.
pub const DOWNLOADS: &str = "https://github.com/samueldotj/bible-compose/releases/download/";

/// How long the question, or the download, may take.
///
/// The installer is tens of megabytes, and the people this is for are often on
/// slow connections. Bounded all the same, because a button nobody can cancel
/// must not be able to wait forever.
const TIMEOUT: Duration = Duration::from_secs(300);

/// What a check found.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum WireUpdate {
    /// This copy was not built to update itself.
    Unavailable,
    /// Nothing newer has been published.
    Current { version: String },
    /// A newer release, which [`install`] will fetch.
    Available {
        version: String,
        current: String,
        notes: Option<String>,
    },
}

/// The update the last check found, kept so that installing it fetches what
/// the person was shown rather than whatever is newest by then.
#[derive(Default)]
pub struct Pending(Mutex<Option<Update>>);

/// Whether this copy carries a key to check an update against.
pub fn can_update<R: Runtime>(app: &AppHandle<R>) -> bool {
    app.config()
        .plugins
        .0
        .get("updater")
        .and_then(|u| u.get("pubkey"))
        .and_then(serde_json::Value::as_str)
        .is_some_and(|key| !key.trim().is_empty())
}

/// Ask [`ENDPOINT`] whether there is anything newer.
pub async fn check<R: Runtime>(
    app: &AppHandle<R>,
    pending: &Pending,
) -> Result<WireUpdate, String> {
    if !can_update(app) {
        return Ok(WireUpdate::Unavailable);
    }
    let endpoint = ENDPOINT.parse().map_err(|e| format!("{e}"))?;
    let updater = app
        .updater_builder()
        .endpoints(vec![endpoint])
        .map_err(|e| e.to_string())?
        .timeout(TIMEOUT)
        .build()
        .map_err(|e| e.to_string())?;

    let found = updater
        .check()
        .await
        .map_err(|e| format!("could not ask GitHub for the latest release: {e}"))?;

    let answer = match &found {
        None => WireUpdate::Current {
            version: app.package_info().version.to_string(),
        },
        Some(update) => {
            allowed(update.download_url.as_str())?;
            WireUpdate::Available {
                version: update.version.clone(),
                current: update.current_version.clone(),
                notes: update.body.clone(),
            }
        }
    };
    *pending.0.lock().expect("the update lock is not poisoned") = found;
    Ok(answer)
}

/// Fetch and install the update [`check`] found, then start the new version.
///
/// The caller has already made sure nothing would be lost by the restart. On
/// Windows the installer replaces the running program and the updater exits
/// on its own; elsewhere the files are swapped in place and this restarts.
pub async fn install<R: Runtime>(app: &AppHandle<R>, pending: &Pending) -> Result<(), String> {
    let update = pending
        .0
        .lock()
        .expect("the update lock is not poisoned")
        .take()
        .ok_or_else(|| "check for updates first".to_owned())?;
    allowed(update.download_url.as_str())?;

    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|e| format!("the update could not be installed: {e}"))?;
    app.restart()
}

/// Refuse a download from anywhere but this project's own releases.
fn allowed(url: &str) -> Result<(), String> {
    if url.starts_with(DOWNLOADS) {
        Ok(())
    } else {
        // A bug or a tampered manifest, and either way not something to
        // fetch. Said plainly, for the same reason `download` says it.
        Err(format!(
            "refusing to download {url}: BibleCompose only installs its own releases from {DOWNLOADS}"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_this_projects_releases_are_downloaded() {
        assert!(allowed(
            "https://github.com/samueldotj/bible-compose/releases/download/v0.5.0/BibleCompose_0.5.0_x64-setup.exe"
        )
        .is_ok());
        for url in [
            "https://github.com/someone/else/releases/download/v1/x.exe",
            "https://github.com.evil.example/samueldotj/bible-compose/releases/download/v1/x.exe",
            "http://github.com/samueldotj/bible-compose/releases/download/v1/x.exe",
            "https://example.com/x.exe",
            "",
        ] {
            assert!(allowed(url).is_err(), "{url}");
        }
    }

    /// The question carries nothing about who is asking: no query string, and
    /// none of the updater's `{{current_version}}`-style placeholders.
    #[test]
    fn the_endpoint_says_nothing_about_the_asker() {
        assert!(ENDPOINT.starts_with(DOWNLOADS.trim_end_matches("download/")));
        assert!(!ENDPOINT.contains('?'));
        assert!(!ENDPOINT.contains("{{"));
    }
}
