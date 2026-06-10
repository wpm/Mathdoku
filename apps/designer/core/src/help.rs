//! Canonical website-documentation links for the in-app Help system
//! (ADR-0007).
//!
//! The app gets tooltips, not documentation: conceptual and workflow content
//! lives on the project website, and the desktop Help menu (built in
//! `src-tauri`) opens these URLs in the system browser. The constants live
//! here — rather than in the Tauri crate — so the link-and-anchor contract is
//! exercised by CI, which tests this crate but cannot compile the Tauri crate
//! (no GTK/WebKit system libraries on the runner).
//!
//! The section anchors listed in [`RULES_ANCHORS`] and [`GUIDE_ANCHORS`] are a
//! contract with the website sources in `site/`: once a shipped app build
//! links to an anchor, renaming or removing it on the website breaks that
//! build's Help menu. The tests below pin every URL and anchor against the
//! HTML the site deploys from.

/// Base URL of the deployed project website (the gh-pages root).
pub const WEBSITE_URL: &str = "https://wpm.github.io/Mathdoku/";

/// The puzzle-rules page: what Mathdoku is, the row/column constraint, and
/// the cage operators and targets. Target of the Help menu's "Puzzle Rules"
/// item.
pub const PUZZLE_RULES_URL: &str = "https://wpm.github.io/Mathdoku/rules/";

/// The Designer-guide page: the authoring workflow from a blank grid to a
/// saved puzzle. Target of the Help menu's "Designer Guide" item.
pub const DESIGNER_GUIDE_URL: &str = "https://wpm.github.io/Mathdoku/guide/";

/// Stable section anchors on the puzzle-rules page. Deep links take the form
/// `{PUZZLE_RULES_URL}#{anchor}`.
pub const RULES_ANCHORS: &[&str] = &["what-is-mathdoku", "rows-and-columns", "cages", "operators"];

/// Stable section anchors on the Designer-guide page. Deep links take the
/// form `{DESIGNER_GUIDE_URL}#{anchor}`.
pub const GUIDE_ANCHORS: &[&str] = &[
    "creating-a-puzzle",
    "building-cages",
    "operators",
    "solvability",
    "saving",
];

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    //! Pins the Help-link contract against the website sources in `site/`.
    //!
    //! These tests read the HTML files the deploy workflow publishes verbatim
    //! to the gh-pages root, so a passing run means every Help menu URL
    //! resolves to a page — and every contractual anchor to an element id —
    //! on the next deployed website.

    use std::fs;
    use std::path::PathBuf;

    use super::{DESIGNER_GUIDE_URL, GUIDE_ANCHORS, PUZZLE_RULES_URL, RULES_ANCHORS, WEBSITE_URL};

    /// Resolves a URL under [`WEBSITE_URL`] to its HTML source in `site/`,
    /// which `deploy-main.yml` publishes unchanged to the gh-pages root.
    fn site_source(url: &str) -> String {
        let path = url.strip_prefix(WEBSITE_URL).unwrap();
        assert!(
            url.ends_with('/'),
            "help URLs must be directory links (GitHub Pages serves index.html): {url}"
        );
        let file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../site")
            .join(path)
            .join("index.html");
        fs::read_to_string(&file)
            .unwrap_or_else(|e| panic!("help link {url} has no site source at {file:?}: {e}"))
    }

    #[test]
    fn help_pages_exist_in_site_sources() {
        for url in [PUZZLE_RULES_URL, DESIGNER_GUIDE_URL] {
            let html = site_source(url);
            assert!(!html.is_empty());
        }
    }

    #[test]
    fn rules_anchors_exist_on_rules_page() {
        let html = site_source(PUZZLE_RULES_URL);
        for anchor in RULES_ANCHORS {
            assert!(
                html.contains(&format!("id=\"{anchor}\"")),
                "rules page is missing contractual anchor #{anchor}"
            );
        }
    }

    #[test]
    fn guide_anchors_exist_on_guide_page() {
        let html = site_source(DESIGNER_GUIDE_URL);
        for anchor in GUIDE_ANCHORS {
            assert!(
                html.contains(&format!("id=\"{anchor}\"")),
                "guide page is missing contractual anchor #{anchor}"
            );
        }
    }

    #[test]
    fn help_pages_are_reachable_from_site_navigation() {
        // ADR-0007 requires the documentation pages be reachable from the
        // site's navigation, not just via deep links from the app.
        let landing = site_source(WEBSITE_URL);
        assert!(landing.contains("href=\"rules/\""));
        assert!(landing.contains("href=\"guide/\""));
    }

    #[test]
    fn help_urls_are_under_the_website_root() {
        for url in [PUZZLE_RULES_URL, DESIGNER_GUIDE_URL] {
            assert!(url.starts_with(WEBSITE_URL));
        }
    }
}
