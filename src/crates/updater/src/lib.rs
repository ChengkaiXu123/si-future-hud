//! SI Future HUD's own updater, a product extension of the host.
//!
//! It reads `update.json` (the official site first, then the copy on GitHub),
//! keeps only a feed whose Ed25519 signature the key compiled in here
//! verifies and whose version is newer than the running one, downloads the
//! disk image to the data folder's `updates/`, checks its size and SHA-256
//! against the feed and the app inside it (bundle id, version), and, when the
//! running copy is an installed one, puts the new app in its place: a small
//! helper waits for this process to quit, swaps the bundles, moves the old
//! one to the Trash (or keeps it in `updates/backups/`) and opens the new one.
//! The host shows what it found through its update notice: the menu bar's
//! New Version… and the HUD footer's arrow, both opening Settings → Updates.
//!
//! Test switches, read only while `SI_HUD_UPDATE_FEED` is set:
//! `SI_HUD_UPDATE_FEED` (the only feed, http(s) or file), which also allows
//! http and file links in it; `SI_HUD_UPDATE_CURRENT_VERSION` (the version to
//! compare with, in place of the running one); `SI_HUD_UPDATE_FIRST_CHECK_SECS`
//! (how long after launch the first automatic check runs).
pub mod feed;
pub mod fetch;
pub mod policy;
mod state;
mod text;
mod updater;

// What differs by OS: where an installed app lives, how its disk image is
// opened and how it is swapped. Only macOS installs updates; elsewhere the
// updater checks and tells.
#[cfg(target_os = "macos")]
#[path = "macos.rs"]
pub mod install;
#[cfg(not(target_os = "macos"))]
#[path = "unsupported.rs"]
pub mod install;

// The whole flow on macOS, with real disk images in temporary folders.
#[cfg(all(test, target_os = "macos"))]
mod tests_macos;

pub use text::Text;
pub use updater::{Config, Quit, Updater};

/// The bundle id every update must carry.
pub const BUNDLE_ID: &str = "com.agenthud.next.preview";
/// Where the official site serves the feed, and the copy on GitHub that is
/// read when the site can't be reached (outside the company network).
pub const FEEDS: [&str; 2] = [
    "https://pages.release.ctripcorp.com/ckxu/si-future-hud/update.json",
    "https://raw.githubusercontent.com/ChengkaiXu123/si-future-hud/main/site/update.json",
];
/// The disk image on the official site (always the latest release).
pub const SITE_DMG: &str =
    "https://pages.release.ctripcorp.com/ckxu/si-future-hud/SI-Future-HUD.dmg";

/// The release's disk image as a GitHub release asset.
pub fn github_dmg(version: &str) -> String {
    format!(
        "https://github.com/ChengkaiXu123/si-future-hud/releases/download/v{version}/SI-Future-HUD_{version}_aarch64.dmg"
    )
}
