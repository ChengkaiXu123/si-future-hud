//! The updater as the host's product extension: one worker that checks on a
//! schedule, downloads and verifies, and installs when asked or, on its own,
//! when no card waits on the notch; Settings → Updates reads and drives it
//! through the extension's requests.
use crate::{
    feed::{self, Feed},
    fetch::{self, Fetcher},
    install, policy,
    state::{self, LastInstall, Phase, Saved},
    text::{text, Text},
    FEEDS,
};
use agent_hud_host::extension::{HostContext, ProductExtension, UpdateNotice};
use serde_json::{json, Value};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, Condvar, Mutex, MutexGuard, OnceLock},
    time::{Duration, SystemTime},
};

/// The Settings page the update notice opens.
const PAGE: &str = "updates";

/// Everything the updater runs with; `for_app` reads the running app's.
pub struct Config {
    /// The version updates are compared with.
    pub current_version: String,
    /// `updates/` in the data folder: state, downloads, the helper and its log.
    pub directory: PathBuf,
    /// The feeds, read in order until one verifies.
    pub sources: Vec<String>,
    /// The test feed (`SI_HUD_UPDATE_FEED`), shown in Settings while it is in use.
    pub test_feed: Option<String>,
    pub first_check: Duration,
    pub every: Duration,
    /// The key release feeds must be signed with.
    pub public_key: Vec<u8>,
    /// The installed app to replace, or why this copy can't be replaced.
    pub install: Result<PathBuf, Text>,
    /// A demo launch, which never checks or installs.
    pub demo: bool,
    /// The process the swap helper waits to quit.
    pub pid: u32,
    pub trash_old: bool,
    pub relaunch: bool,
    pub wait_for_quit: Duration,
}

impl Config {
    /// The running app's: `version`, the data folder, the official feeds (or
    /// the test feed) and the compiled-in key.
    pub fn for_app(version: &str) -> Result<Self, String> {
        let test_feed = std::env::var("SI_HUD_UPDATE_FEED")
            .ok()
            .map(|feed| feed.trim().to_owned())
            .filter(|feed| !feed.is_empty())
            .map(|feed| {
                // A plain path is taken as a local file.
                if feed.contains("://") {
                    feed
                } else {
                    reqwest::Url::from_file_path(&feed)
                        .map(|url| url.to_string())
                        .unwrap_or(feed)
                }
            });
        // The other switches count only with a test feed.
        let switch = |name: &str| {
            test_feed
                .as_ref()
                .and_then(|_| std::env::var(name).ok())
                .map(|value| value.trim().to_owned())
                .filter(|value| !value.is_empty())
        };
        let current_version =
            switch("SI_HUD_UPDATE_CURRENT_VERSION").unwrap_or_else(|| version.to_owned());
        let first_check = switch("SI_HUD_UPDATE_FIRST_CHECK_SECS")
            .and_then(|seconds| seconds.parse().ok())
            .map(Duration::from_secs)
            .unwrap_or(policy::FIRST_CHECK);
        let directory = agent_hud_runtime::local_tools_directory()?.join("updates");
        let home = agent_hud_runtime::home_directory()?;
        let install = std::env::current_exe()
            .map_err(|error| text(error.to_string(), error.to_string()))
            .and_then(|executable| {
                install::install_target(&executable, &home, cfg!(debug_assertions))
            });
        let demo =
            std::env::args().any(|argument| argument == "--demo" || argument == "--demo-script");
        Ok(Self {
            current_version,
            directory,
            sources: match &test_feed {
                Some(feed) => vec![feed.clone()],
                None => FEEDS.iter().map(|feed| (*feed).to_owned()).collect(),
            },
            test_feed,
            first_check,
            every: policy::EVERY,
            public_key: feed::embedded_public_key(),
            install,
            demo,
            pid: std::process::id(),
            trash_old: true,
            relaunch: true,
            wait_for_quit: Duration::from_secs(120),
        })
    }
}

/// How the updater quits the app once the swap helper waits for it.
pub type Quit = Arc<dyn Fn() + Send + Sync>;

/// The product extension. `Updater::new()` reads its configuration when the
/// host starts it.
pub struct Updater {
    inner: Arc<Inner>,
}

struct Inner {
    config: OnceLock<Config>,
    /// The app's version, from the Tauri context.
    app_version: OnceLock<String>,
    /// Quits the app once the helper waits for it.
    quit: OnceLock<Quit>,
    host: OnceLock<HostContext>,
    shared: Mutex<Shared>,
    wake: Condvar,
}

#[derive(Default)]
struct Shared {
    saved: Saved,
    phase: Phase,
    progress: Option<(u64, u64)>,
    /// Why the last check, download or install failed.
    error: Option<Text>,
    /// The downloaded, verified disk image of the offered version.
    ready: Option<Ready>,
    last_install: Option<LastInstall>,
    /// No card waits on the notch (`policy::idle`).
    idle: bool,
    asked_check: bool,
    asked_install: bool,
    stopped: bool,
}

#[derive(Clone)]
struct Ready {
    version: String,
    image: PathBuf,
}

enum Job {
    Check,
    Install { asked: bool },
}

impl Default for Updater {
    fn default() -> Self {
        Self::new()
    }
}

impl Updater {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Inner {
                config: OnceLock::new(),
                app_version: OnceLock::new(),
                quit: OnceLock::new(),
                host: OnceLock::new(),
                shared: Mutex::new(Shared::default()),
                wake: Condvar::new(),
            }),
        }
    }

    /// An updater with its configuration and its way to quit given (tests).
    pub fn with_config(config: Config, quit: Quit) -> Self {
        let updater = Self::new();
        let _ = updater.inner.config.set(config);
        let _ = updater.inner.quit.set(quit);
        updater
    }
}

impl ProductExtension for Updater {
    fn setup_native(&self, app: &tauri::AppHandle) -> Result<(), String> {
        let _ = self
            .inner
            .app_version
            .set(app.package_info().version.to_string());
        let app = app.clone();
        let _ = self.inner.quit.set(Arc::new(move || app.exit(0)));
        Ok(())
    }

    fn start(&self, host: HostContext) -> Result<(), String> {
        let _ = self.inner.host.set(host);
        if self.inner.config.get().is_none() {
            let version = self
                .inner
                .app_version
                .get()
                .cloned()
                .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_owned());
            match Config::for_app(&version) {
                Ok(config) => {
                    let _ = self.inner.config.set(config);
                }
                // The app runs without updates rather than not at all.
                Err(error) => {
                    eprintln!("SI Future HUD update: not started: {error}");
                    return Ok(());
                }
            }
        }
        self.inner.restore();
        let inner = Arc::clone(&self.inner);
        std::thread::Builder::new()
            .name("si-hud-updater".into())
            .spawn(move || inner.run())
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    fn request(&self, method: &str, params: Value) -> Result<Value, String> {
        match method {
            // The local tools channel asks every product for its tools; the
            // updater has none.
            "host_tools" => return Ok(json!({"tools": []})),
            "host_tool" => return Err("Unknown local application tool".into()),
            _ => {}
        }
        let inner = &self.inner;
        let config = inner
            .config
            .get()
            .ok_or_else(|| l10n("更新程序没有运行", "The updater is not running"))?;
        let view = {
            let mut shared = inner.lock();
            match method {
                "updates.state" => {}
                "updates.check" => {
                    shared.asked_check = true;
                }
                "updates.install" => {
                    if !can_install(&shared, config) {
                        return Err(l10n(
                            "还没有下载好、可以安装的新版本",
                            "No downloaded update is ready to install",
                        ));
                    }
                    shared.asked_install = true;
                }
                "updates.set" => {
                    let value = params["value"]
                        .as_bool()
                        .ok_or("The value must be true or false")?;
                    match params["key"].as_str() {
                        Some("autoCheck") => shared.saved.auto_check = value,
                        Some("autoInstall") => shared.saved.auto_install = value,
                        _ => return Err("Unknown update setting".into()),
                    }
                    inner.persist(&shared.saved);
                }
                _ => return Err(format!("Unknown request: {method}")),
            }
            inner.wake.notify_all();
            inner.view(&shared)
        };
        if method != "updates.state" {
            inner.publish(view.clone(), None);
        }
        Ok(view)
    }

    fn observe_snapshot(&self, snapshot: Arc<Value>) {
        let idle = policy::idle(&snapshot);
        let mut shared = self.inner.lock();
        if shared.idle != idle {
            shared.idle = idle;
            self.inner.wake.notify_all();
        }
    }

    fn stop(&self) {
        self.inner.lock().stopped = true;
        self.inner.wake.notify_all();
    }
}

fn l10n(zh: &str, en: &str) -> String {
    agent_hud_runtime::l10n::text(zh, en)
}

fn can_install(shared: &Shared, config: &Config) -> bool {
    shared.ready.is_some()
        && config.install.is_ok()
        && !config.demo
        && shared.phase != Phase::Installing
}

impl Inner {
    fn config(&self) -> &Config {
        self.config
            .get()
            .expect("the updater runs with its configuration")
    }

    fn lock(&self) -> MutexGuard<'_, Shared> {
        self.shared
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn state_path(&self) -> PathBuf {
        self.config().directory.join("state.json")
    }

    fn persist(&self, saved: &Saved) {
        if let Err(error) = state::save(&self.state_path(), saved) {
            self.log(&format!("state not saved: {error}"));
        }
    }

    /// A line in `updates/update.log` (kept under 512 KB) and on stderr.
    fn log(&self, line: &str) {
        eprintln!("SI Future HUD update: {line}");
        let Some(config) = self.config.get() else {
            return;
        };
        let path = config.directory.join("update.log");
        if fs::metadata(&path).is_ok_and(|meta| meta.len() > 512 * 1024) {
            let _ = fs::rename(&path, config.directory.join("update.log.1"));
        }
        let _ = fs::create_dir_all(&config.directory);
        if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(&path) {
            let _ = writeln!(file, "{} {line}", chrono::Utc::now().to_rfc3339());
        }
    }

    /// At start: the saved state, what the last swap left, and the files an
    /// earlier version no longer needs.
    fn restore(&self) {
        let config = self.config();
        let (view, notice) = {
            let mut shared = self.lock();
            shared.saved = state::load(&self.state_path());
            let result = config.directory.join("install-result");
            if let Ok(line) = fs::read_to_string(&result) {
                let _ = fs::remove_file(&result);
                if let Some(last) = state::read_install_result(&line) {
                    self.log(&last.message.en);
                    if !last.ok {
                        shared.saved.failed_version = Some(last.version.clone());
                    }
                    shared.last_install = Some(last);
                }
            }
            let current = &config.current_version;
            let still_newer = |version: &str| policy::newer(version, current).unwrap_or(false);
            if shared
                .saved
                .offer
                .as_ref()
                .is_some_and(|offer| !still_newer(&offer.version))
            {
                shared.saved.offer = None;
            }
            if shared
                .saved
                .failed_version
                .as_ref()
                .is_some_and(|version| !still_newer(version))
            {
                shared.saved.failed_version = None;
            }
            if shared.saved.offer.is_some() {
                shared.phase = Phase::Available;
            }
            let keep = shared
                .saved
                .offer
                .as_ref()
                .map(|offer| fetch::image_name(&offer.version));
            tidy(&config.directory, keep.as_deref());
            self.persist(&shared.saved);
            (self.view(&shared), notice(&shared, config))
        };
        self.publish(view, Some(notice));
    }

    fn run(self: Arc<Self>) {
        let config = self.config();
        let automatic = !config.demo;
        // By the wall clock, so the time a Mac sleeps counts toward the next
        // check; each wait is short enough to notice the Mac woke.
        let mut next_check = SystemTime::now() + config.first_check;
        loop {
            self.refresh_idle();
            let job = {
                let mut shared = self.lock();
                loop {
                    if shared.stopped {
                        return;
                    }
                    if shared.asked_check {
                        shared.asked_check = false;
                        break Job::Check;
                    }
                    if shared.ready.is_some() {
                        let facts = policy::InstallFacts {
                            asked: shared.asked_install,
                            automatic: shared.saved.auto_install,
                            installable: config.install.is_ok(),
                            idle: shared.idle,
                            failed_before: shared.saved.failed_version.as_deref()
                                == shared.ready.as_ref().map(|ready| ready.version.as_str()),
                            demo: config.demo,
                        };
                        if policy::install_now(facts) {
                            let asked = std::mem::take(&mut shared.asked_install);
                            break Job::Install { asked };
                        }
                    }
                    shared.asked_install = false;
                    let checking = automatic && shared.saved.auto_check;
                    let due = policy::until(SystemTime::now(), next_check);
                    if checking && due.is_none() {
                        break Job::Check;
                    }
                    let mut wait = match due {
                        Some(due) if checking => due,
                        _ => policy::LONGEST_WAIT,
                    }
                    .min(policy::LONGEST_WAIT);
                    // A downloaded update waiting for a quiet notch looks again
                    // now and then, as well as at every new snapshot.
                    if shared.ready.is_some() && shared.saved.auto_install {
                        wait = wait.min(policy::IDLE_RECHECK);
                    }
                    shared = match self.wake.wait_timeout(shared, wait) {
                        Ok((shared, timeout)) => {
                            if timeout.timed_out() {
                                // Out to read the snapshot again.
                                drop(shared);
                                self.refresh_idle();
                                self.lock()
                            } else {
                                shared
                            }
                        }
                        Err(_) => return,
                    };
                }
            };
            match job {
                Job::Check => {
                    next_check = SystemTime::now() + config.every;
                    self.check();
                }
                Job::Install { asked } => self.install(asked),
            }
        }
    }

    /// Reads whether a card waits from the host's snapshot.
    fn refresh_idle(&self) {
        let Some(host) = self.host.get() else {
            return;
        };
        if let Ok(snapshot) = (host.snapshot)() {
            self.lock().idle = policy::idle(&snapshot);
        }
    }

    /// Changes the state under its lock, then tells Settings and the host.
    fn change(&self, edit: impl FnOnce(&mut Shared, &Config)) {
        let config = self.config();
        let (view, notice) = {
            let mut shared = self.lock();
            edit(&mut shared, config);
            (self.view(&shared), notice(&shared, config))
        };
        self.publish(view, Some(notice));
    }

    fn publish(&self, view: Value, notice: Option<Option<UpdateNotice>>) {
        let Some(host) = self.host.get() else {
            return;
        };
        if let Some(notice) = notice {
            (host.update)(notice);
        }
        (host.publish)(json!({ "updates": view }));
    }

    fn check(&self) {
        let config = self.config();
        self.change(|shared, _| {
            shared.phase = Phase::Checking;
            shared.error = None;
            shared.progress = None;
        });
        let fetched =
            Fetcher::new(&config.current_version, config.test_feed.is_some()).and_then(|fetcher| {
                let (feed, source) = fetcher.feed(&config.sources, &config.public_key)?;
                Ok((fetcher, feed, source))
            });
        let checked_at = chrono::Utc::now().to_rfc3339();
        let (fetcher, feed, source) = match fetched {
            Ok(found) => found,
            Err(reason) => {
                self.log(&format!("check failed: {reason}"));
                return self.change(|shared, _| {
                    shared.saved.last_check_at = Some(checked_at);
                    self.persist(&shared.saved);
                    shared.phase = settled(shared);
                    shared.error = Some(text(
                        format!("检查更新失败：{}", reason.zh),
                        format!("Checking for updates failed: {}", reason.en),
                    ));
                });
            }
        };
        let newer = policy::newer(&feed.version, &config.current_version);
        self.log(&format!(
            "{source} offers {} (running {}): {}",
            feed.version,
            config.current_version,
            match &newer {
                Ok(true) => "newer".to_owned(),
                Ok(false) => "not newer".to_owned(),
                Err(reason) => reason.en.clone(),
            }
        ));
        match newer {
            Err(reason) => {
                return self.change(|shared, _| {
                    shared.saved.last_check_at = Some(checked_at);
                    self.persist(&shared.saved);
                    shared.phase = settled(shared);
                    shared.error = Some(reason);
                })
            }
            Ok(false) => {
                tidy(&config.directory, None);
                return self.change(|shared, _| {
                    shared.saved.last_check_at = Some(checked_at);
                    shared.saved.offer = None;
                    shared.saved.failed_version = None;
                    self.persist(&shared.saved);
                    shared.ready = None;
                    shared.phase = Phase::UpToDate;
                });
            }
            Ok(true) => {}
        }
        let mut downloaded = false;
        self.change(|shared, config| {
            shared.saved.last_check_at = Some(checked_at);
            if shared.saved.failed_version.as_deref() != Some(feed.version.as_str()) {
                shared.saved.failed_version = None;
            }
            shared.saved.offer = Some(feed.clone());
            self.persist(&shared.saved);
            if shared
                .ready
                .as_ref()
                .is_some_and(|ready| ready.version == feed.version && ready.image.is_file())
            {
                shared.phase = Phase::Ready;
                downloaded = true;
            } else if config.install.is_err() || config.demo {
                // Only told: this copy can't replace itself.
                shared.ready = None;
                shared.phase = Phase::Available;
                downloaded = true;
            } else {
                shared.ready = None;
                shared.phase = Phase::Downloading;
                shared.progress = Some((0, feed.size));
            }
        });
        if downloaded {
            return;
        }
        let mut shown = 0;
        let image = fetcher.download(&feed, &config.directory, &mut |done, total| {
            let percent = done.saturating_mul(100) / total.max(1);
            if percent != shown {
                shown = percent;
                self.change(|shared, _| shared.progress = Some((done, total)));
            }
        });
        let verified = image.and_then(|image| match install::inspect(&image, &feed.version) {
            Ok(()) => Ok(image),
            Err(reason) => {
                let _ = fs::remove_file(&image);
                Err(reason)
            }
        });
        match verified {
            Ok(image) => {
                self.log(&format!(
                    "{} downloaded and verified: {}",
                    feed.version,
                    image.display()
                ));
                tidy(&config.directory, Some(&fetch::image_name(&feed.version)));
                self.change(|shared, _| {
                    shared.progress = None;
                    shared.ready = Some(Ready {
                        version: feed.version.clone(),
                        image,
                    });
                    shared.phase = Phase::Ready;
                });
            }
            Err(reason) => {
                self.log(&format!("{} abandoned: {reason}", feed.version));
                self.change(|shared, _| {
                    shared.progress = None;
                    shared.phase = Phase::Failed;
                    shared.error = Some(text(
                        format!("已放弃这次更新：{}", reason.zh),
                        format!("This update was abandoned: {}", reason.en),
                    ));
                });
            }
        }
    }

    fn install(&self, asked: bool) {
        let config = self.config();
        let (Some(ready), Ok(app)) = (self.lock().ready.clone(), config.install.clone()) else {
            return;
        };
        self.log(&format!(
            "installing {} over {} ({})",
            ready.version,
            app.display(),
            if asked { "asked" } else { "automatic" }
        ));
        self.change(|shared, _| {
            shared.phase = Phase::Installing;
            shared.error = None;
        });
        let previous =
            install::installed_version(&app).unwrap_or_else(|| config.current_version.clone());
        let launched = install::stage(&ready.image, &ready.version, &app).and_then(|staged| {
            let swap = install::Swap {
                pid: config.pid,
                app: app.clone(),
                staged: staged.clone(),
                version: ready.version.clone(),
                previous,
                script: config.directory.join("install.sh"),
                backups: config.directory.join("backups"),
                result: config.directory.join("install-result"),
                log: config.directory.join("update.log"),
                trash: config.trash_old,
                relaunch: config.relaunch,
                wait: config.wait_for_quit,
            };
            install::launch_swap(&swap).inspect_err(|_| {
                let _ = fs::remove_dir_all(&staged);
            })
        });
        match launched {
            Ok(_helper) => {
                self.log("the helper waits for the app to quit");
                // Nothing more runs here: the app quits and the helper opens
                // the new one.
                self.lock().stopped = true;
                if let Some(quit) = self.quit.get() {
                    quit();
                }
            }
            Err(reason) => {
                self.log(&format!("install of {} failed: {reason}", ready.version));
                self.change(|shared, _| {
                    shared.ready = None;
                    shared.saved.failed_version = Some(ready.version.clone());
                    self.persist(&shared.saved);
                    shared.phase = Phase::Failed;
                    shared.error = Some(text(
                        format!("安装没有完成：{}", reason.zh),
                        format!("The install did not finish: {}", reason.en),
                    ));
                });
            }
        }
    }

    fn view(&self, shared: &Shared) -> Value {
        let config = self.config();
        let offer: Option<&Feed> = shared.saved.offer.as_ref();
        // Installs on its own at the next quiet moment (a version that failed
        // before waits for the user instead).
        let waiting = shared.phase == Phase::Ready
            && shared.saved.auto_install
            && !shared.idle
            && config.install.is_ok()
            && !config.demo
            && shared.saved.failed_version.as_deref()
                != shared.ready.as_ref().map(|ready| ready.version.as_str());
        let blocked = if config.demo {
            Some(text(
                "演示模式不检查也不安装更新",
                "The demo neither checks for nor installs updates",
            ))
        } else {
            config.install.as_ref().err().cloned()
        };
        json!({
            "currentVersion": config.current_version,
            "autoCheck": shared.saved.auto_check,
            "autoInstall": shared.saved.auto_install,
            "lastCheckAt": shared.saved.last_check_at,
            "phase": shared.phase,
            "status": state::status(
                shared.phase,
                offer.map(|offer| offer.version.as_str()),
                shared.progress,
                waiting,
            ),
            "progress": shared.progress.map(|(done, total)| json!({"done": done, "total": total})),
            "error": shared.error,
            "available": offer.map(|offer| json!({
                "version": offer.version,
                "pubDate": offer.pub_date,
                "notes": offer.notes,
                "size": offer.size,
            })),
            "canCheck": !config.demo
                && !matches!(shared.phase, Phase::Checking | Phase::Downloading | Phase::Installing),
            "canInstall": can_install(shared, config),
            "blocked": blocked,
            "waitingForIdle": waiting,
            "testFeed": config.test_feed,
            "lastInstall": shared.last_install,
        })
    }
}

/// The phase after a check that didn't change anything: what was there.
fn settled(shared: &Shared) -> Phase {
    if shared.ready.is_some() {
        Phase::Ready
    } else if shared.saved.offer.is_some() {
        Phase::Available
    } else {
        Phase::Failed
    }
}

/// The host's update notice: the offered version while it is newer than the
/// running one.
fn notice(shared: &Shared, config: &Config) -> Option<UpdateNotice> {
    shared
        .saved
        .offer
        .as_ref()
        .filter(|offer| policy::newer(&offer.version, &config.current_version).unwrap_or(false))
        .map(|offer| UpdateNotice {
            version: offer.version.clone(),
            settings_page: PAGE.into(),
        })
}

/// Removes disk images other than `keep`, unfinished downloads, and all but
/// the newest kept old copy.
fn tidy(directory: &Path, keep: Option<&str>) {
    if let Ok(entries) = fs::read_dir(directory) {
        for entry in entries.filter_map(Result::ok) {
            let name = entry.file_name().to_string_lossy().into_owned();
            let image = name.starts_with("SI-Future-HUD-")
                && (name.ends_with(".dmg") || name.ends_with(".dmg.part"));
            if image && Some(name.as_str()) != keep {
                let _ = fs::remove_file(entry.path());
            }
        }
    }
    if let Ok(entries) = fs::read_dir(directory.join("backups")) {
        let mut copies: Vec<(std::time::SystemTime, PathBuf)> = entries
            .filter_map(Result::ok)
            .filter_map(|entry| Some((entry.metadata().ok()?.modified().ok()?, entry.path())))
            .collect();
        copies.sort();
        copies.pop();
        for (_, path) in copies {
            let _ = fs::remove_dir_all(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tidying_keeps_the_offered_image_and_the_newest_old_copy() {
        let folder = tempfile::tempdir().unwrap();
        let directory = folder.path();
        for name in [
            "SI-Future-HUD-0.1.8.dmg",
            "SI-Future-HUD-0.1.7.dmg",
            "SI-Future-HUD-0.1.9.dmg.part",
            "state.json",
            "update.log",
        ] {
            fs::write(directory.join(name), b"x").unwrap();
        }
        let backups = directory.join("backups");
        fs::create_dir_all(backups.join("SI Future HUD 0.1.6.app")).unwrap();
        std::thread::sleep(Duration::from_millis(20));
        fs::create_dir_all(backups.join("SI Future HUD 0.1.7.app")).unwrap();
        tidy(directory, Some("SI-Future-HUD-0.1.8.dmg"));
        let mut left: Vec<String> = fs::read_dir(directory)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        left.sort();
        assert_eq!(
            left,
            [
                "SI-Future-HUD-0.1.8.dmg",
                "backups",
                "state.json",
                "update.log"
            ]
        );
        assert!(backups.join("SI Future HUD 0.1.7.app").exists());
        assert!(!backups.join("SI Future HUD 0.1.6.app").exists());
    }
}
