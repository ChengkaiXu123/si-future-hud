//! Systems other than macOS: not implemented. The updater still checks and
//! shows the update notice; the release is a macOS disk image, so nothing is
//! downloaded or installed here.
use crate::text::{text, Text};
use std::{
    path::{Path, PathBuf},
    process::Child,
    time::Duration,
};

fn unsupported() -> Text {
    text(
        "这个系统还不支持自动安装更新，请从官网下载新版本",
        "Updates are not installed automatically on this system; download the new version from the website",
    )
}

pub fn install_target(
    _executable: &Path,
    _home: &Path,
    _development: bool,
) -> Result<PathBuf, Text> {
    Err(unsupported())
}

pub fn installed_version(_app: &Path) -> Option<String> {
    None
}

pub fn inspect(_dmg: &Path, _version: &str) -> Result<(), Text> {
    Err(unsupported())
}

pub fn stage(_dmg: &Path, _version: &str, _app: &Path) -> Result<PathBuf, Text> {
    Err(unsupported())
}

pub fn staged_path(_app: &Path, _version: &str) -> Result<PathBuf, Text> {
    Err(unsupported())
}

pub struct Swap {
    pub pid: u32,
    pub app: PathBuf,
    pub staged: PathBuf,
    pub version: String,
    pub previous: String,
    pub script: PathBuf,
    pub backups: PathBuf,
    pub result: PathBuf,
    pub log: PathBuf,
    pub trash: bool,
    pub relaunch: bool,
    pub wait: Duration,
}

pub fn launch_swap(_swap: &Swap) -> Result<Child, Text> {
    Err(unsupported())
}
