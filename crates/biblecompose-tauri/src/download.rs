//! The one place this application opens a socket (FONT-006).
//!
//! BibleCompose composes offline and says so on its own installer. That
//! remains true of everything it does with a project: loading, validating,
//! composing and writing a PDF reach the network nowhere (NFR-004), and the
//! offline end-to-end test is what keeps it true rather than this comment.
//!
//! Fetching a font is the single exception, and it is an exception a person
//! asks for by clicking a button about a named family. So the egress is here
//! — in the shell, which is already the layer that owns dialogs, the
//! filesystem and the browser — rather than in `biblecompose-app`, where an
//! HTTP client would sit in the same crate as the build. That is why
//! [`biblecompose_app::typeface::package`] takes bytes instead of a URL: the
//! layering is what makes "the build never reaches the network" a fact about
//! the dependency graph instead of a promise about intent.
//!
//! # What is and is not allowed out
//!
//! One host, from the catalogue, checked here on every request. A URL that
//! does not begin with it is refused without a connection being made — so a
//! mistake in the catalogue, or a future row somebody adds carelessly, cannot
//! turn this into a general-purpose fetcher. Nothing is ever sent *out*: no
//! query string, no identifying header beyond what the request needs, no
//! cookies, and the response is bytes that go to a checksum.

use std::time::Duration;

use biblecompose_app::typeface;

/// How long one file may take before the attempt is abandoned.
///
/// Generous, because the largest file in the catalogue is a few megabytes and
/// the people this is for are often on a connection that makes that slow.
/// Bounded, because a window with no cancel button must not be able to wait
/// forever — and a font download is not worth one.
const TIMEOUT: Duration = Duration::from_secs(120);

/// Fetch one file, refusing anything but the catalogue's host.
///
/// `expected` is the byte count the catalogue records, and the read stops one
/// byte past it: a response longer than the file it claims to be is wrong
/// however it got that way, and there is no reason to buffer the rest of it to
/// find out. The caller verifies the exact length and the digest, which is
/// where a short read is caught — this only declines to be used as a sink.
pub fn fetch(url: &str, expected: usize) -> Result<Vec<u8>, String> {
    let prefix = format!("{}/{}/", typeface::HOST, typeface::REPOSITORY);
    if !url.starts_with(&prefix) {
        // Not a message a publisher is meant to act on. It means the catalogue
        // and this check disagree, which is a bug, and saying so plainly beats
        // a cheerful "download failed".
        return Err(format!(
            "refusing to fetch {url}: BibleCompose only ever contacts {}",
            typeface::HOST,
        ));
    }

    let agent = ureq::Agent::new_with_config(
        ureq::Agent::config_builder()
            .timeout_global(Some(TIMEOUT))
            .build(),
    );

    let mut response = agent
        .get(url)
        .call()
        .map_err(|e| format!("could not reach {}: {e}", typeface::HOST))?;

    let status = response.status();
    if !status.is_success() {
        return Err(format!(
            "{} answered {status} for {}",
            typeface::HOST,
            name_of(url),
        ));
    }

    response
        .body_mut()
        .with_config()
        .limit(expected as u64 + 1)
        .read_to_vec()
        .map_err(|e| format!("could not read {}: {e}", name_of(url)))
}

/// The file name at the end of a URL, for a message that names the file rather
/// than repeating a hundred characters of path.
fn name_of(url: &str) -> &str {
    url.rsplit('/').next().unwrap_or(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The allow-list is checked before anything is dialled, so this test makes
    /// no connection — which is the property being asserted.
    #[test]
    fn only_the_catalogue_host_is_fetched_from() {
        for url in [
            "https://example.com/evil.ttf",
            "http://raw.githubusercontent.com/google/fonts/x/ofl/a/b.ttf",
            "https://raw.githubusercontent.com/someone/else/x/b.ttf",
            "file:///etc/passwd",
            "",
        ] {
            let refused = fetch(url, 10).expect_err("refused");
            assert!(refused.contains("only ever contacts"), "{url}: {refused}");
        }
    }

    /// And the real catalogue's own URLs pass it, so the check and the table
    /// cannot drift apart without a test saying so.
    #[test]
    fn every_catalogue_url_passes_the_allow_list() {
        let prefix = format!("{}/{}/", typeface::HOST, typeface::REPOSITORY);
        for entry in typeface::TYPEFACES {
            assert!(entry.licence_url().starts_with(&prefix), "{}", entry.family);
            for file in entry.files {
                let url = entry.url(file.name);
                assert!(url.starts_with(&prefix), "{url}");
                // The brackets of a variable font's name must be escaped by the
                // time they reach a client that validates its input.
                assert!(!url.contains('['), "{url}");
            }
        }
    }
}
