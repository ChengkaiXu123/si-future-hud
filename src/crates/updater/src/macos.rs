//! macOS: where an installed copy lives, what is inside a release's disk
//! image, and the swap of one app bundle for another.
use crate::text::{text, Text};
use crate::BUNDLE_ID;
use std::{
    fs,
    os::unix::{fs::PermissionsExt, process::CommandExt},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::Duration,
};

/// The app bundle this copy may replace with an update: the running
/// executable's `.app`, when it is installed in `/Applications` or the
/// user's `~/Applications`. Never a development build, a copy running from
/// its disk image, or the random read-only copy macOS runs a quarantined app
/// from (App Translocation); those only hear that an update exists.
pub fn install_target(executable: &Path, home: &Path, development: bool) -> Result<PathBuf, Text> {
    let development_build = || {
        text(
            "这是开发构建，不会自动安装更新",
            "This is a development build; it does not install updates",
        )
    };
    if development {
        return Err(development_build());
    }
    let bundle = executable
        .parent()
        .filter(|folder| folder.file_name().is_some_and(|name| name == "MacOS"))
        .and_then(Path::parent)
        .filter(|folder| folder.file_name().is_some_and(|name| name == "Contents"))
        .and_then(Path::parent)
        .filter(|folder| {
            folder
                .extension()
                .is_some_and(|extension| extension == "app")
        })
        .ok_or_else(development_build)?;
    let path = executable.to_string_lossy();
    if path.contains("/AppTranslocation/")
        || path.starts_with("/private/var/folders/")
        || path.starts_with("/var/folders/")
    {
        return Err(text(
            "macOS 正从临时的隔离位置运行这个副本：把 SI Future HUD 拖进“应用程序”再打开，就能自动更新",
            "macOS runs this copy from a temporary, isolated place: move SI Future HUD to Applications and open it there to get updates installed",
        ));
    }
    if path.starts_with("/Volumes/") {
        return Err(text(
            "正在从磁盘映像或外接磁盘运行：把 SI Future HUD 拖进“应用程序”再打开，就能自动更新",
            "SI Future HUD runs from a disk image or another volume: move it to Applications and open it there to get updates installed",
        ));
    }
    if bundle.starts_with("/Applications") || bundle.starts_with(home.join("Applications")) {
        return Ok(bundle.to_path_buf());
    }
    let place = bundle
        .parent()
        .map(|folder| folder.display().to_string())
        .unwrap_or_default();
    Err(text(
        format!("只有装在“应用程序”文件夹里的副本会自动安装更新（这个副本在 {place}）"),
        format!("Only a copy in Applications installs updates (this one is in {place})"),
    ))
}

/// The installed app's own version (CFBundleShortVersionString).
pub fn installed_version(app: &Path) -> Option<String> {
    bundle_info(app).ok().map(|(_, version)| version)
}

/// Whether the release's disk image holds one app, SI Future HUD's bundle id,
/// at `version`.
pub fn inspect(dmg: &Path, version: &str) -> Result<(), Text> {
    let mounted = attach(dmg)?;
    let app = find_app(mounted.path())?;
    check_bundle(&app, version)
}

/// Copies the app out of the disk image to a hidden name beside `app` (one
/// volume, so the swap is a rename), checked again after the copy. Answers
/// the copy.
pub fn stage(dmg: &Path, version: &str, app: &Path) -> Result<PathBuf, Text> {
    let staged = staged_path(app, version)?;
    let _ = fs::remove_dir_all(&staged);
    let copied = {
        let mounted = attach(dmg)?;
        let source = find_app(mounted.path())?;
        check_bundle(&source, version)?;
        Command::new("/usr/bin/ditto")
            .arg(&source)
            .arg(&staged)
            .output()
    };
    let folder = app.parent().unwrap_or(Path::new("/")).display().to_string();
    match copied {
        Ok(output) if output.status.success() => {}
        Ok(output) => {
            let _ = fs::remove_dir_all(&staged);
            let error = String::from_utf8_lossy(&output.stderr).trim().to_owned();
            return Err(text(
                format!("无法把新版本复制到 {folder}：{error}"),
                format!("The new version could not be copied to {folder}: {error}"),
            ));
        }
        Err(error) => {
            let _ = fs::remove_dir_all(&staged);
            return Err(text(
                format!("无法运行 ditto：{error}"),
                format!("ditto could not run: {error}"),
            ));
        }
    }
    if let Err(reason) = check_bundle(&staged, version) {
        let _ = fs::remove_dir_all(&staged);
        return Err(reason);
    }
    // Copied by this app from a verified image, never downloaded by a
    // browser: nothing for Gatekeeper to ask about.
    let _ = Command::new("/usr/bin/xattr")
        .args(["-dr", "com.apple.quarantine"])
        .arg(&staged)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    Ok(staged)
}

/// The hidden folder an update is staged in beside `app`: not a `.app`, so
/// Launch Services never lists it.
pub fn staged_path(app: &Path, version: &str) -> Result<PathBuf, Text> {
    let stem = app
        .file_stem()
        .ok_or_else(|| text("安装位置无效", "The install location is invalid"))?;
    let folder = app
        .parent()
        .ok_or_else(|| text("安装位置无效", "The install location is invalid"))?;
    Ok(folder.join(format!(".{} {version}.update", stem.to_string_lossy())))
}

/// What the swap helper needs.
pub struct Swap {
    /// The running app's process, which the helper waits to quit.
    pub pid: u32,
    /// The installed app, replaced in place.
    pub app: PathBuf,
    /// The new app, staged beside it (`stage`).
    pub staged: PathBuf,
    pub version: String,
    /// The installed app's version, for the old copy's name in the Trash.
    pub previous: String,
    /// The helper script, written here.
    pub script: PathBuf,
    /// Where the old copy goes when it can't go to the Trash.
    pub backups: PathBuf,
    /// The helper's one-line result, for the next launch.
    pub result: PathBuf,
    /// The helper's own log.
    pub log: PathBuf,
    /// Move the old copy to the Trash (tests keep it in `backups`).
    pub trash: bool,
    /// Open the app when done (tests don't).
    pub relaunch: bool,
    /// How long to wait for the running app to quit.
    pub wait: Duration,
}

/// Starts the helper, detached from this process (its own process group,
/// so it outlives the app), and returns at once: the app quits next.
pub fn launch_swap(swap: &Swap) -> Result<Child, Text> {
    let fail = |error: std::io::Error| {
        text(
            format!("无法启动安装助手：{error}"),
            format!("The install helper could not start: {error}"),
        )
    };
    if let Some(folder) = swap.script.parent() {
        fs::create_dir_all(folder).map_err(fail)?;
    }
    fs::write(&swap.script, SWAP_SCRIPT).map_err(fail)?;
    fs::set_permissions(&swap.script, fs::Permissions::from_mode(0o700)).map_err(fail)?;
    let _ = fs::remove_file(&swap.result);
    let log = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&swap.log)
        .map_err(fail)?;
    let tenths = (swap.wait.as_millis() / 100).max(1).to_string();
    Command::new("/bin/sh")
        .arg(&swap.script)
        .arg(swap.pid.to_string())
        .arg(&swap.app)
        .arg(&swap.staged)
        .arg(&swap.version)
        .arg(&swap.previous)
        .arg(&swap.backups)
        .arg(&swap.result)
        .arg(if swap.trash { "1" } else { "0" })
        .arg(if swap.relaunch { "1" } else { "0" })
        .arg(tenths)
        .stdin(Stdio::null())
        .stdout(log.try_clone().map_err(fail)?)
        .stderr(log)
        .process_group(0)
        .spawn()
        .map_err(fail)
}

/// The swap, run after the app quits. The old app is moved aside first and
/// the new one renamed into its place; only then does the old one go to the
/// backups folder under its own name and on to the Trash. Any failure puts
/// the old app back and opens it again, so the user is never left without
/// the app.
const SWAP_SCRIPT: &str = r#"#!/bin/sh
# SI Future HUD's update helper, written by the app before it quits
# (src/crates/updater/src/macos.rs).
trap '' HUP
pid=$1 app=$2 staged=$3 version=$4 previous=$5 backups=$6 result=$7 trash=$8 relaunch=$9
shift 9
wait_tenths=${1:-1200}
name=$(basename "$app" .app)
say() { printf '%s update helper: %s\n' "$(date '+%Y-%m-%dT%H:%M:%S%z')" "$*"; }
finish() {
  printf '%s\t%s\t%s\n' "$1" "$version" "$2" > "$result.part" && mv -f "$result.part" "$result"
  say "$1 $version: $2"
}
reopen() { if [ "$relaunch" = 1 ]; then /usr/bin/open "$app"; fi; }
fail() { rm -rf "$staged"; finish failed "$1"; reopen; exit 1; }

say "installing $version over $previous at $app; waiting for process $pid to quit"
n=0
while kill -0 "$pid" 2>/dev/null; do
  n=$((n + 1))
  if [ "$n" -gt "$wait_tenths" ]; then
    rm -rf "$staged"
    finish failed "the running copy did not quit"
    exit 1
  fi
  sleep 0.1
done
[ -d "$staged" ] || fail "the new copy is missing"
aside="$(dirname "$app")/.$name $previous.previous"
rm -rf "$aside"
mv "$app" "$aside" || fail "could not move the old copy aside"
if ! mv "$staged" "$app"; then
  mv "$aside" "$app"
  fail "could not move the new copy into place"
fi
mkdir -p "$backups"
old="$backups/$name $previous.app"
[ -e "$old" ] && old="$backups/$name $previous $(date +%Y%m%d-%H%M%S).app"
where="$aside"
if mv "$aside" "$old"; then
  where="$old"
  if [ "$trash" = 1 ]; then
    if [ -x /usr/bin/trash ]; then
      /usr/bin/trash -s "$old" >/dev/null 2>&1
    elif [ -d "$HOME/.Trash" ]; then
      target="$HOME/.Trash/$(basename "$old")"
      [ -e "$target" ] && target="$HOME/.Trash/$name $previous $(date +%Y%m%d-%H%M%S).app"
      mv "$old" "$target"
    fi
    [ -e "$old" ] || where="Trash"
  fi
fi
finish ok "$where"
reopen
exit 0
"#;

fn bundle_info(app: &Path) -> Result<(String, String), Text> {
    let unreadable = |reason: String| {
        text(
            format!("读不了 app 的 Info.plist：{reason}"),
            format!("The app's Info.plist could not be read: {reason}"),
        )
    };
    let info = plist::Value::from_file(app.join("Contents").join("Info.plist"))
        .map_err(|error| unreadable(error.to_string()))?;
    let info = info
        .as_dictionary()
        .ok_or_else(|| unreadable("not a dictionary".into()))?;
    let field = |key: &str| {
        info.get(key)
            .and_then(plist::Value::as_string)
            .unwrap_or("")
            .to_owned()
    };
    Ok((
        field("CFBundleIdentifier"),
        field("CFBundleShortVersionString"),
    ))
}

/// The app must be SI Future HUD (its bundle id) at the feed's version.
pub fn check_bundle(app: &Path, version: &str) -> Result<(), Text> {
    let (id, found) = bundle_info(app)?;
    if id != BUNDLE_ID {
        return Err(text(
            format!("磁盘映像里的 app 不是 SI Future HUD（bundle id 是 {id}）"),
            format!("The app in the disk image is not SI Future HUD (its bundle id is {id})"),
        ));
    }
    if found != version {
        return Err(text(
            format!("磁盘映像里的 app 是 {found} 版，不是 update.json 写的 {version}"),
            format!(
                "The app in the disk image is version {found}, not the {version} update.json names"
            ),
        ));
    }
    Ok(())
}

/// A disk image attached read-only at a private mount point, hidden from
/// the Finder; detached when dropped.
struct Mounted {
    point: Option<tempfile::TempDir>,
}

impl Mounted {
    fn path(&self) -> &Path {
        self.point
            .as_ref()
            .map_or(Path::new("/"), |point| point.path())
    }
}

impl Drop for Mounted {
    fn drop(&mut self) {
        let Some(point) = self.point.take() else {
            return;
        };
        let detach = |force: bool| {
            let mut command = Command::new("/usr/bin/hdiutil");
            command.arg("detach").arg(point.path()).arg("-quiet");
            if force {
                command.arg("-force");
            }
            command
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .is_ok_and(|status| status.success())
        };
        if detach(false) || {
            std::thread::sleep(Duration::from_secs(1));
            detach(true)
        } {
            drop(point);
        } else {
            // Still mounted: leave the mount point alone rather than delete
            // through it.
            eprintln!(
                "SI Future HUD update: {} could not be detached",
                point.keep().display()
            );
        }
    }
}

fn attach(dmg: &Path) -> Result<Mounted, Text> {
    let point = tempfile::Builder::new()
        .prefix("si-hud-update-")
        .tempdir()
        .map_err(|error| {
            text(
                format!("无法准备挂载点：{error}"),
                format!("No mount point could be made: {error}"),
            )
        })?;
    let output = Command::new("/usr/bin/hdiutil")
        .args([
            "attach",
            "-nobrowse",
            "-readonly",
            "-noautoopen",
            "-mountpoint",
        ])
        .arg(point.path())
        .arg(dmg)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| {
            text(
                format!("无法运行 hdiutil：{error}"),
                format!("hdiutil could not run: {error}"),
            )
        })?;
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(text(
            format!("磁盘映像打不开：{error}"),
            format!("The disk image could not be opened: {error}"),
        ));
    }
    Ok(Mounted { point: Some(point) })
}

/// The one app at the top of the image (its Applications link is skipped).
fn find_app(root: &Path) -> Result<PathBuf, Text> {
    let apps: Vec<PathBuf> = fs::read_dir(root)
        .map_err(|error| {
            text(
                format!("读不了磁盘映像：{error}"),
                format!("The disk image could not be read: {error}"),
            )
        })?
        .filter_map(Result::ok)
        .filter(|entry| {
            entry.file_type().is_ok_and(|kind| kind.is_dir())
                && entry
                    .path()
                    .extension()
                    .is_some_and(|extension| extension == "app")
        })
        .map(|entry| entry.path())
        .collect();
    match apps.as_slice() {
        [app] => Ok(app.clone()),
        [] => Err(text("磁盘映像里没有 app", "The disk image holds no app")),
        _ => Err(text(
            "磁盘映像里有不止一个 app",
            "The disk image holds more than one app",
        )),
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A stand-in app bundle: an Info.plist and an executable file.
    pub(crate) fn fake_app(path: &Path, id: &str, version: &str) {
        let contents = path.join("Contents");
        fs::create_dir_all(contents.join("MacOS")).unwrap();
        fs::write(contents.join("MacOS").join("agent-hud-next"), version).unwrap();
        let mut info = plist::Dictionary::new();
        info.insert("CFBundleIdentifier".into(), id.into());
        info.insert("CFBundleShortVersionString".into(), version.into());
        info.insert("CFBundleVersion".into(), version.into());
        plist::Value::Dictionary(info)
            .to_file_xml(contents.join("Info.plist"))
            .unwrap();
    }

    /// A release disk image the way the build makes one: the app and a link
    /// to /Applications.
    pub(crate) fn fake_dmg(folder: &Path, id: &str, version: &str) -> PathBuf {
        let source = folder.join(format!("image-{version}-{id}"));
        fake_app(&source.join("SI Future HUD.app"), id, version);
        std::os::unix::fs::symlink("/Applications", source.join("Applications")).unwrap();
        let dmg = folder.join(format!("SI-Future-HUD-{version}-{id}.dmg"));
        let status = Command::new("/usr/bin/hdiutil")
            .args([
                "create",
                "-quiet",
                "-ov",
                "-format",
                "UDZO",
                "-volname",
                "SI Future HUD",
                "-srcfolder",
            ])
            .arg(&source)
            .arg(&dmg)
            .status()
            .unwrap();
        assert!(status.success(), "hdiutil create");
        dmg
    }

    #[test]
    fn only_an_installed_copy_installs_updates() {
        let home = Path::new("/Users/someone");
        let installed = Path::new("/Applications/SI Future HUD.app/Contents/MacOS/agent-hud-next");
        assert_eq!(
            install_target(installed, home, false).unwrap(),
            Path::new("/Applications/SI Future HUD.app")
        );
        let personal = Path::new(
            "/Users/someone/Applications/SI Future HUD.app/Contents/MacOS/agent-hud-next",
        );
        assert_eq!(
            install_target(personal, home, false).unwrap(),
            Path::new("/Users/someone/Applications/SI Future HUD.app")
        );
        let nested =
            Path::new("/Applications/Tools/SI Future HUD.app/Contents/MacOS/agent-hud-next");
        assert!(install_target(nested, home, false).is_ok());

        let refused = |path: &str, development: bool| {
            install_target(Path::new(path), home, development)
                .unwrap_err()
                .en
        };
        // Development builds: `cargo run`, and a debug binary in a bundle.
        assert!(refused("/repo/target/release/agent-hud-next", false).contains("development build"));
        assert!(refused("/repo/target/debug/agent-hud-next", false).contains("development build"));
        assert!(refused(installed.to_str().unwrap(), true).contains("development build"));
        // A release bundle that was never moved to Applications.
        assert!(refused(
            "/repo/target/release/bundle/macos/SI Future HUD.app/Contents/MacOS/agent-hud-next",
            false
        )
        .contains("Only a copy in Applications"));
        assert!(refused(
            "/Users/someone/Downloads/SI Future HUD.app/Contents/MacOS/agent-hud-next",
            false
        )
        .contains("Only a copy in Applications"));
        // Its disk image, and App Translocation.
        assert!(refused(
            "/Volumes/SI Future HUD/SI Future HUD.app/Contents/MacOS/agent-hud-next",
            false
        )
        .contains("disk image"));
        assert!(refused(
            "/private/var/folders/ab/cd/T/AppTranslocation/1234/d/SI Future HUD.app/Contents/MacOS/agent-hud-next",
            false
        )
        .contains("temporary, isolated place"));
        assert!(refused(
            "/ApplicationsCopy/SI Future HUD.app/Contents/MacOS/agent-hud-next",
            false
        )
        .contains("Only a copy in Applications"));
    }

    #[test]
    fn an_app_must_carry_the_bundle_id_and_the_feeds_version() {
        let folder = tempfile::tempdir().unwrap();
        let app = folder.path().join("SI Future HUD.app");
        fake_app(&app, BUNDLE_ID, "0.1.8");
        check_bundle(&app, "0.1.8").unwrap();
        assert_eq!(installed_version(&app).as_deref(), Some("0.1.8"));
        assert!(check_bundle(&app, "0.1.9")
            .unwrap_err()
            .en
            .contains("version 0.1.8, not the 0.1.9"));
        let other = folder.path().join("Other.app");
        fake_app(&other, "com.example.other", "0.1.8");
        assert!(check_bundle(&other, "0.1.8")
            .unwrap_err()
            .en
            .contains("com.example.other"));
        assert!(check_bundle(&folder.path().join("Missing.app"), "0.1.8").is_err());
    }

    #[test]
    fn a_disk_image_is_inspected_and_staged_beside_the_installed_app() {
        let folder = tempfile::tempdir().unwrap();
        let good = fake_dmg(folder.path(), BUNDLE_ID, "0.1.8");
        inspect(&good, "0.1.8").unwrap();
        assert!(inspect(&good, "0.1.9")
            .unwrap_err()
            .en
            .contains("not the 0.1.9"));
        let stranger = fake_dmg(folder.path(), "com.example.other", "0.1.8");
        assert!(inspect(&stranger, "0.1.8")
            .unwrap_err()
            .en
            .contains("not SI Future HUD"));
        let broken = folder.path().join("broken.dmg");
        fs::write(&broken, b"not a disk image").unwrap();
        assert!(inspect(&broken, "0.1.8")
            .unwrap_err()
            .en
            .contains("could not be opened"));

        let applications = folder.path().join("Applications");
        let installed = applications.join("SI Future HUD.app");
        fake_app(&installed, BUNDLE_ID, "0.1.7");
        let staged = stage(&good, "0.1.8", &installed).unwrap();
        assert_eq!(staged, applications.join(".SI Future HUD 0.1.8.update"));
        check_bundle(&staged, "0.1.8").unwrap();
        check_bundle(&installed, "0.1.7").unwrap();
        assert!(stage(&stranger, "0.1.8", &installed).is_err());
        assert!(
            !applications.join(".SI Future HUD 0.1.8.update").exists(),
            "nothing left behind"
        );
        // Every image was detached: no mount point of ours is left.
        let mounts =
            String::from_utf8(Command::new("/sbin/mount").output().unwrap().stdout).unwrap();
        assert!(!mounts.contains(folder.path().to_str().unwrap()));
    }

    fn swap(folder: &Path, pid: u32, wait: Duration) -> Swap {
        let applications = folder.join("Applications");
        let updates = folder.join("updates");
        Swap {
            pid,
            app: applications.join("SI Future HUD.app"),
            staged: applications.join(".SI Future HUD 0.1.8.update"),
            version: "0.1.8".into(),
            previous: "0.1.7".into(),
            script: updates.join("install.sh"),
            backups: updates.join("backups"),
            result: updates.join("install-result"),
            log: updates.join("update.log"),
            trash: false,
            relaunch: false,
            wait,
        }
    }

    #[test]
    fn the_helper_swaps_the_apps_once_the_app_quits_and_keeps_the_old_one() {
        let folder = tempfile::tempdir().unwrap();
        let plan = swap(folder.path(), 0, Duration::from_secs(10));
        fake_app(&plan.app, BUNDLE_ID, "0.1.7");
        fake_app(&plan.staged, BUNDLE_ID, "0.1.8");
        fs::create_dir_all(plan.log.parent().unwrap()).unwrap();
        let mut running = Command::new("/bin/sleep").arg("0.5").spawn().unwrap();
        let plan = Swap {
            pid: running.id(),
            ..plan
        };
        let mut helper = launch_swap(&plan).unwrap();
        // The helper waits for "the app" to quit.
        std::thread::sleep(Duration::from_millis(200));
        assert!(!plan.result.exists(), "it waits while the app runs");
        check_bundle(&plan.app, "0.1.7").unwrap();
        running.wait().unwrap();
        assert!(helper.wait().unwrap().success());
        check_bundle(&plan.app, "0.1.8").unwrap();
        assert!(!plan.staged.exists());
        let old = plan.backups.join("SI Future HUD 0.1.7.app");
        check_bundle(&old, "0.1.7").unwrap();
        assert_eq!(
            fs::read_to_string(&plan.result).unwrap(),
            format!("ok\t0.1.8\t{}\n", old.display())
        );
        let applications: Vec<_> = fs::read_dir(plan.app.parent().unwrap())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(
            applications,
            vec!["SI Future HUD.app"],
            "nothing hidden left"
        );
        assert!(fs::read_to_string(&plan.log).unwrap().contains("ok 0.1.8"));
    }

    #[test]
    fn the_helper_leaves_the_old_app_in_place_when_it_cant_swap() {
        // The new copy is gone by the time the app quits.
        let folder = tempfile::tempdir().unwrap();
        let plan = swap(folder.path(), 0, Duration::from_secs(10));
        fake_app(&plan.app, BUNDLE_ID, "0.1.7");
        fs::create_dir_all(plan.log.parent().unwrap()).unwrap();
        let mut running = Command::new("/bin/sleep").arg("0.1").spawn().unwrap();
        let plan = Swap {
            pid: running.id(),
            ..plan
        };
        let mut helper = launch_swap(&plan).unwrap();
        running.wait().unwrap();
        assert!(!helper.wait().unwrap().success());
        check_bundle(&plan.app, "0.1.7").unwrap();
        assert_eq!(
            fs::read_to_string(&plan.result).unwrap(),
            "failed\t0.1.8\tthe new copy is missing\n"
        );

        // The app never quits: the helper gives up and removes the copy.
        let folder = tempfile::tempdir().unwrap();
        let plan = swap(
            folder.path(),
            std::process::id(),
            Duration::from_millis(300),
        );
        fake_app(&plan.app, BUNDLE_ID, "0.1.7");
        fake_app(&plan.staged, BUNDLE_ID, "0.1.8");
        fs::create_dir_all(plan.log.parent().unwrap()).unwrap();
        let mut helper = launch_swap(&plan).unwrap();
        assert!(!helper.wait().unwrap().success());
        check_bundle(&plan.app, "0.1.7").unwrap();
        assert!(!plan.staged.exists());
        assert_eq!(
            fs::read_to_string(&plan.result).unwrap(),
            "failed\t0.1.8\tthe running copy did not quit\n"
        );
    }
}
