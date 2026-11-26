//! What the updater keeps between launches (`updates/state.json`), what the
//! swap helper leaves for the next launch (`updates/install-result`), and
//! the status line Settings shows.
use crate::feed::Feed;
use crate::text::{text, Text};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

fn on() -> bool {
    true
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Saved {
    /// Check for updates automatically.
    #[serde(default = "on")]
    pub auto_check: bool,
    /// Install new versions automatically.
    #[serde(default = "on")]
    pub auto_install: bool,
    /// The last check, finished or failed (RFC 3339).
    #[serde(default)]
    pub last_check_at: Option<String>,
    /// The newest verified feed the last good check found newer than the
    /// running version.
    #[serde(default)]
    pub offer: Option<Feed>,
    /// A version whose install failed: it is not installed automatically
    /// again, only when the user asks.
    #[serde(default)]
    pub failed_version: Option<String>,
}

impl Default for Saved {
    fn default() -> Self {
        Self {
            auto_check: true,
            auto_install: true,
            last_check_at: None,
            offer: None,
            failed_version: None,
        }
    }
}

/// The saved state, or the defaults when there is none or it can't be read.
pub(crate) fn load(path: &Path) -> Saved {
    fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

pub(crate) fn save(path: &Path, saved: &Saved) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let partial = path.with_extension("json.part");
    fs::write(&partial, serde_json::to_vec_pretty(saved)?)?;
    fs::rename(partial, path)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum Phase {
    /// No check yet since launch.
    #[default]
    Idle,
    Checking,
    UpToDate,
    /// A newer version, not downloaded (this copy can't install it, or the
    /// download hasn't started).
    Available,
    Downloading,
    /// Downloaded and verified; it installs when asked or, automatically,
    /// when the notch is quiet.
    Ready,
    Installing,
    Failed,
}

/// How the last swap went, as the helper wrote it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LastInstall {
    pub ok: bool,
    pub version: String,
    pub message: Text,
}

/// The helper's one line: `ok|failed <TAB> version <TAB> where the old copy
/// went, or why it failed`.
pub(crate) fn read_install_result(line: &str) -> Option<LastInstall> {
    let mut fields = line.trim_end().splitn(3, '\t');
    let outcome = fields.next()?;
    let version = fields.next()?.to_owned();
    let detail = fields.next().unwrap_or("").to_owned();
    match outcome {
        "ok" => {
            let message = if detail == "Trash" {
                text(
                    format!("已更新到 {version}，旧版本在废纸篓里，需要时可以拖回“应用程序”。"),
                    format!("Updated to {version}. The previous version is in the Trash if you need it back."),
                )
            } else {
                text(
                    format!("已更新到 {version}，旧版本保存在 {detail}。"),
                    format!("Updated to {version}. The previous version is kept in {detail}."),
                )
            };
            Some(LastInstall {
                ok: true,
                version,
                message,
            })
        }
        "failed" => {
            let reason = match detail.as_str() {
                "the running copy did not quit" => "旧版本没有退出",
                "the new copy is missing" => "新版本的文件不见了",
                "could not move the old copy aside" => "无法挪开旧版本",
                "could not move the new copy into place" => "无法把新版本放到原来的位置",
                other => other,
            };
            Some(LastInstall {
                ok: false,
                message: text(
                    format!("安装 {version} 没有成功：{reason}。旧版本留在原处。"),
                    format!("Installing {version} failed: {detail}. The previous version stayed in place."),
                ),
                version,
            })
        }
        _ => None,
    }
}

/// The status line: what the updater is doing, or found.
pub(crate) fn status(
    phase: Phase,
    offered: Option<&str>,
    progress: Option<(u64, u64)>,
    waiting_for_idle: bool,
) -> Text {
    let version = offered.unwrap_or("");
    match phase {
        Phase::Idle => text("还没有检查过", "Not checked yet"),
        Phase::Checking => text("正在检查更新…", "Checking for updates…"),
        Phase::UpToDate => text("已是最新版本", "SI Future HUD is up to date"),
        Phase::Available => text(
            format!("发现新版本 {version}"),
            format!("Version {version} is available"),
        ),
        Phase::Downloading => {
            let percent = progress
                .filter(|(_, total)| *total > 0)
                .map(|(done, total)| done.saturating_mul(100) / total)
                .unwrap_or(0);
            text(
                format!("正在下载 {version}（{percent}%）"),
                format!("Downloading {version} ({percent}%)"),
            )
        }
        Phase::Ready if waiting_for_idle => text(
            format!("{version} 已下载并校验，等刘海上的卡片处理完就安装并重启"),
            format!("{version} is downloaded and verified; it installs and restarts once no card waits on the notch"),
        ),
        Phase::Ready => text(
            format!("{version} 已下载并校验，可以安装"),
            format!("{version} is downloaded, verified and ready to install"),
        ),
        Phase::Installing => text(
            format!("正在安装 {version}，马上重启…"),
            format!("Installing {version}; SI Future HUD restarts in a moment…"),
        ),
        // The error line under it says what went wrong.
        Phase::Failed => text(
            "上次检查或更新没有完成",
            "The last check or update did not finish",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_state_defaults_to_both_switches_on_and_survives_a_round_trip() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("updates").join("state.json");
        assert_eq!(load(&path), Saved::default());
        assert!(Saved::default().auto_check && Saved::default().auto_install);
        let saved = Saved {
            auto_install: false,
            last_check_at: Some("2026-11-26T08:00:00Z".into()),
            ..Saved::default()
        };
        save(&path, &saved).unwrap();
        assert_eq!(load(&path), saved);
        fs::write(&path, b"not json").unwrap();
        assert_eq!(load(&path), Saved::default(), "a broken file is not fatal");
        // An older file without the switches turns them on.
        fs::write(&path, br#"{"lastCheckAt":null}"#).unwrap();
        assert!(load(&path).auto_check);
    }

    #[test]
    fn the_helpers_result_reads_in_both_languages() {
        let trashed = read_install_result("ok\t0.1.8\tTrash\n").unwrap();
        assert!(trashed.ok);
        assert_eq!(trashed.version, "0.1.8");
        assert!(trashed.message.zh.contains("废纸篓"));
        let kept = read_install_result("ok\t0.1.8\t/x/backups/SI Future HUD 0.1.7.app").unwrap();
        assert!(kept
            .message
            .en
            .contains("/x/backups/SI Future HUD 0.1.7.app"));
        let failed =
            read_install_result("failed\t0.1.8\tcould not move the old copy aside").unwrap();
        assert!(!failed.ok);
        assert!(failed.message.zh.contains("无法挪开旧版本"));
        assert!(failed
            .message
            .en
            .contains("could not move the old copy aside"));
        assert_eq!(read_install_result(""), None);
        assert_eq!(read_install_result("maybe\t0.1.8\t"), None);
    }

    #[test]
    fn the_status_line_follows_the_phase() {
        assert_eq!(
            status(Phase::Downloading, Some("0.1.8"), Some((25, 100)), false).zh,
            "正在下载 0.1.8（25%）"
        );
        assert_eq!(
            status(Phase::Downloading, Some("0.1.8"), None, false).en,
            "Downloading 0.1.8 (0%)"
        );
        assert!(status(Phase::Ready, Some("0.1.8"), None, true)
            .zh
            .contains("卡片处理完"));
        assert!(status(Phase::Ready, Some("0.1.8"), None, false)
            .en
            .contains("ready to install"));
        assert_eq!(
            status(Phase::UpToDate, None, None, false).zh,
            "已是最新版本"
        );
    }
}
