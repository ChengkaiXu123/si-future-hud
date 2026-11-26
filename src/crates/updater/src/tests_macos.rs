//! The updater end to end on macOS, in temporary folders: a signed test feed
//! and disk image on disk, a stand-in installed app, and a stand-in for the
//! running app's process that "quits" when the updater asks.
use crate::{
    feed::{self, Feed, Notes},
    fetch,
    install::tests::{fake_app, fake_dmg},
    policy, Config, Text, Updater, BUNDLE_ID,
};
use agent_hud_host::extension::{HostContext, ProductExtension, UpdateNotice};
use serde_json::{json, Value};
use std::{
    path::{Path, PathBuf},
    process::{Child, Command},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

struct Rig {
    folder: tempfile::TempDir,
    app: PathBuf,
    feed: PathBuf,
    private: Vec<u8>,
    public: Vec<u8>,
}

impl Rig {
    fn new() -> Self {
        let folder = tempfile::tempdir().unwrap();
        let app = folder.path().join("Applications").join("SI Future HUD.app");
        fake_app(&app, BUNDLE_ID, "0.1.7");
        let (private, public) = feed::generate_key().unwrap();
        let feed = folder.path().join("update.json");
        Self {
            folder,
            app,
            feed,
            private,
            public,
        }
    }

    /// Publishes `image` as `version` (signed with the test key).
    fn publish(&self, version: &str, image: &Path) {
        let sha256 = fetch::sha256_file(image).unwrap();
        let made = Feed {
            version: version.into(),
            pub_date: Some("2026-11-26T08:00:00Z".into()),
            notes: Notes {
                zh: "测试版本".into(),
                en: "A test release".into(),
            },
            size: std::fs::metadata(image).unwrap().len(),
            urls: vec![reqwest::Url::from_file_path(image).unwrap().to_string()],
            signature: feed::sign(version, &sha256, &self.private).unwrap(),
            sha256,
        };
        std::fs::write(&self.feed, serde_json::to_vec(&made).unwrap()).unwrap();
    }

    fn config(&self, install: Result<PathBuf, Text>, pid: u32) -> Config {
        Config {
            current_version: "0.1.7".into(),
            directory: self.folder.path().join("data").join("updates"),
            sources: vec![reqwest::Url::from_file_path(&self.feed)
                .unwrap()
                .to_string()],
            test_feed: Some("test".into()),
            first_check: Duration::from_millis(50),
            every: policy::EVERY,
            public_key: self.public.clone(),
            install,
            demo: false,
            pid,
            trash_old: false,
            relaunch: false,
            wait_for_quit: Duration::from_secs(20),
        }
    }
}

/// What the host saw: notices, and the snapshot the updater reads.
#[derive(Default)]
struct Host {
    notices: Mutex<Vec<Option<UpdateNotice>>>,
    snapshot: Mutex<Value>,
}

fn host(seen: &Arc<Host>) -> HostContext {
    let snapshot = Arc::clone(seen);
    let notices = Arc::clone(seen);
    HostContext {
        snapshot: Arc::new(move || Ok(snapshot.snapshot.lock().unwrap().clone())),
        query: Arc::new(|_, _| Err("none".into())),
        answer: Arc::new(|_, _| Err("none".into())),
        publish: Arc::new(|_| {}),
        fly: Arc::new(|_| {}),
        update: Arc::new(move |notice| notices.notices.lock().unwrap().push(notice)),
    }
}

fn state(updater: &Updater) -> Value {
    updater.request("updates.state", Value::Null).unwrap()
}

fn wait_for(updater: &Updater, what: &str, done: impl Fn(&Value) -> bool) -> Value {
    let until = Instant::now() + Duration::from_secs(60);
    loop {
        let view = state(updater);
        if done(&view) {
            return view;
        }
        assert!(
            Instant::now() < until,
            "timed out waiting for {what}: {view:#}"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// A stand-in for the running app: the helper waits for it to quit.
fn running() -> Child {
    Command::new("/bin/sleep").arg("120").spawn().unwrap()
}

#[test]
fn an_update_waits_for_a_quiet_notch_then_replaces_the_installed_app() {
    let rig = Rig::new();
    let image = fake_dmg(rig.folder.path(), BUNDLE_ID, "0.1.8");
    rig.publish("0.1.8", &image);
    let process = Arc::new(Mutex::new(running()));
    let pid = process.lock().unwrap().id();
    let quitting = Arc::clone(&process);
    let quit = Arc::new(move || {
        let mut app = quitting.lock().unwrap();
        let _ = app.kill();
        let _ = app.wait();
    });
    let updater = Updater::with_config(rig.config(Ok(rig.app.clone()), pid), quit);
    let seen = Arc::new(Host::default());
    let card = json!({"approvals": [{"id": "hook:claude:1:1", "kind": "permission"}]});
    *seen.snapshot.lock().unwrap() = card.clone();
    updater.start(host(&seen)).unwrap();
    updater.observe_snapshot(Arc::new(card));

    // Checked, downloaded and verified on its own; a card waits, so it
    // doesn't install.
    let view = wait_for(&updater, "the download", |view| view["phase"] == "ready");
    assert_eq!(view["available"]["version"], "0.1.8");
    assert_eq!(view["available"]["notes"]["zh"], "测试版本");
    assert_eq!(view["waitingForIdle"], true);
    assert_eq!(view["canInstall"], true);
    assert!(view["lastCheckAt"].is_string());
    assert!(view["status"]["zh"]
        .as_str()
        .unwrap()
        .contains("卡片处理完"));
    assert_eq!(
        seen.notices.lock().unwrap().last().cloned().flatten(),
        Some(UpdateNotice {
            version: "0.1.8".into(),
            settings_page: "updates".into()
        })
    );
    std::thread::sleep(Duration::from_millis(500));
    assert!(
        process.lock().unwrap().try_wait().unwrap().is_none(),
        "not quit yet"
    );
    crate::install::check_bundle(&rig.app, "0.1.7").unwrap();

    // The card is answered: it installs, quits, and the helper swaps.
    *seen.snapshot.lock().unwrap() = json!({"approvals": []});
    updater.observe_snapshot(Arc::new(json!({"approvals": []})));
    let result = rig.folder.path().join("data/updates/install-result");
    let until = Instant::now() + Duration::from_secs(60);
    while !result.exists() {
        assert!(Instant::now() < until, "the helper never finished");
        std::thread::sleep(Duration::from_millis(50));
    }
    let line = std::fs::read_to_string(&result).unwrap();
    assert!(line.starts_with("ok\t0.1.8\t"), "{line}");
    crate::install::check_bundle(&rig.app, "0.1.8").unwrap();
    let kept = rig
        .folder
        .path()
        .join("data/updates/backups/SI Future HUD 0.1.7.app");
    crate::install::check_bundle(&kept, "0.1.7").unwrap();
    updater.stop();

    // The next launch, now 0.1.8, reports the update and drops the offer.
    let next = Updater::with_config(
        Config {
            current_version: "0.1.8".into(),
            first_check: Duration::from_secs(3600),
            ..rig.config(Ok(rig.app.clone()), pid)
        },
        Arc::new(|| {}),
    );
    let seen = Arc::new(Host::default());
    next.start(host(&seen)).unwrap();
    let view = state(&next);
    assert_eq!(view["lastInstall"]["ok"], true);
    assert!(view["lastInstall"]["message"]["zh"]
        .as_str()
        .unwrap()
        .contains("已更新到 0.1.8"));
    assert!(view["available"].is_null());
    assert_eq!(seen.notices.lock().unwrap().last().cloned(), Some(None));
    assert!(!result.exists(), "read once");
    next.stop();
}

#[test]
fn a_copy_that_isnt_installed_only_tells() {
    let rig = Rig::new();
    let image = fake_dmg(rig.folder.path(), BUNDLE_ID, "0.1.8");
    rig.publish("0.1.8", &image);
    let blocked = Text::new("这是开发构建", "This is a development build");
    let updater = Updater::with_config(rig.config(Err(blocked), 0), Arc::new(|| panic!("no quit")));
    let seen = Arc::new(Host::default());
    *seen.snapshot.lock().unwrap() = json!({"approvals": []});
    updater.start(host(&seen)).unwrap();
    let view = wait_for(&updater, "the check", |view| view["phase"] == "available");
    assert_eq!(view["blocked"]["en"], "This is a development build");
    assert_eq!(view["canInstall"], false);
    assert!(
        updater.request("updates.install", Value::Null).is_err(),
        "nothing to install"
    );
    assert!(
        !rig.folder
            .path()
            .join("data/updates/SI-Future-HUD-0.1.8.dmg")
            .exists(),
        "nothing downloaded"
    );
    assert_eq!(
        seen.notices
            .lock()
            .unwrap()
            .last()
            .cloned()
            .flatten()
            .map(|notice| notice.version),
        Some("0.1.8".into()),
        "the menu bar and the HUD still tell"
    );
    updater.stop();
}

#[test]
fn an_update_that_fails_a_check_is_abandoned_with_its_reason() {
    // An image whose app is 0.1.9, published as 0.1.8.
    let rig = Rig::new();
    let image = fake_dmg(rig.folder.path(), BUNDLE_ID, "0.1.9");
    rig.publish("0.1.8", &image);
    let updater = Updater::with_config(
        rig.config(Ok(rig.app.clone()), 0),
        Arc::new(|| panic!("no quit")),
    );
    let seen = Arc::new(Host::default());
    *seen.snapshot.lock().unwrap() = json!({"approvals": []});
    updater.start(host(&seen)).unwrap();
    let view = wait_for(&updater, "the check", |view| view["phase"] == "failed");
    let reason = view["error"]["en"].as_str().unwrap();
    assert!(
        reason.contains("abandoned") && reason.contains("version 0.1.9"),
        "{reason}"
    );
    assert!(!rig
        .folder
        .path()
        .join("data/updates/SI-Future-HUD-0.1.8.dmg")
        .exists());
    crate::install::check_bundle(&rig.app, "0.1.7").unwrap();

    // A feed signed with another key: refused before anything downloads.
    let (stranger, _) = feed::generate_key().unwrap();
    let good = fake_dmg(rig.folder.path(), BUNDLE_ID, "0.1.8");
    let sha256 = fetch::sha256_file(&good).unwrap();
    let mut forged: Feed = serde_json::from_slice(&std::fs::read(&rig.feed).unwrap()).unwrap();
    forged.sha256 = sha256.clone();
    forged.size = std::fs::metadata(&good).unwrap().len();
    forged.urls = vec![reqwest::Url::from_file_path(&good).unwrap().to_string()];
    forged.signature = feed::sign("0.1.8", &sha256, &stranger).unwrap();
    std::fs::write(&rig.feed, serde_json::to_vec(&forged).unwrap()).unwrap();
    let before = state(&updater)["lastCheckAt"].clone();
    updater.request("updates.check", Value::Null).unwrap();
    let view = wait_for(&updater, "the second check", |view| {
        view["lastCheckAt"] != before && view["phase"] != "checking"
    });
    assert!(
        view["error"]["en"]
            .as_str()
            .unwrap()
            .contains("Signature check failed"),
        "{view:#}"
    );
    assert!(!rig
        .folder
        .path()
        .join("data/updates/SI-Future-HUD-0.1.8.dmg")
        .exists());
    updater.stop();
}

#[test]
fn switches_are_saved_and_turning_off_automatic_install_leaves_it_to_the_user() {
    let rig = Rig::new();
    let image = fake_dmg(rig.folder.path(), BUNDLE_ID, "0.1.8");
    rig.publish("0.1.8", &image);
    let quit = Arc::new(Mutex::new(false));
    let quitting = Arc::clone(&quit);
    let mut config = rig.config(Ok(rig.app.clone()), 0);
    config.first_check = Duration::from_secs(3600);
    let updater = Updater::with_config(config, Arc::new(move || *quitting.lock().unwrap() = true));
    let seen = Arc::new(Host::default());
    *seen.snapshot.lock().unwrap() = json!({"approvals": []});
    updater.start(host(&seen)).unwrap();
    assert_eq!(state(&updater)["phase"], "idle");
    let view = updater
        .request("updates.set", json!({"key": "autoInstall", "value": false}))
        .unwrap();
    assert_eq!(view["autoInstall"], false);
    let saved: Value = serde_json::from_slice(
        &std::fs::read(rig.folder.path().join("data/updates/state.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(saved["autoInstall"], false);
    assert!(updater
        .request("updates.set", json!({"key": "autoInstall", "value": "no"}))
        .is_err());

    // Checked by hand: downloaded and ready, and it waits for the user even
    // with a quiet notch.
    updater.request("updates.check", Value::Null).unwrap();
    let view = wait_for(&updater, "the download", |view| view["phase"] == "ready");
    assert_eq!(view["waitingForIdle"], false);
    std::thread::sleep(Duration::from_millis(300));
    assert!(!*quit.lock().unwrap());
    updater.stop();
}

#[test]
fn a_failed_install_is_not_retried_on_its_own() {
    use std::os::unix::fs::PermissionsExt;
    let rig = Rig::new();
    let image = fake_dmg(rig.folder.path(), BUNDLE_ID, "0.1.8");
    rig.publish("0.1.8", &image);
    // Applications can't be written: the new copy can't be staged.
    let applications = rig.app.parent().unwrap().to_path_buf();
    std::fs::set_permissions(&applications, std::fs::Permissions::from_mode(0o555)).unwrap();
    let updater = Updater::with_config(
        rig.config(Ok(rig.app.clone()), 0),
        Arc::new(|| panic!("no quit")),
    );
    let seen = Arc::new(Host::default());
    *seen.snapshot.lock().unwrap() = json!({"approvals": []});
    updater.start(host(&seen)).unwrap();
    let view = wait_for(&updater, "the failed install", |view| {
        view["phase"] == "failed"
    });
    let reason = view["error"]["en"].as_str().unwrap().to_owned();
    assert!(reason.contains("could not be copied"), "{reason}");
    crate::install::check_bundle(&rig.app, "0.1.7").unwrap();

    // Checked again, it is ready but waits for the user, quiet notch or not.
    updater.request("updates.check", Value::Null).unwrap();
    let view = wait_for(&updater, "the second check", |view| {
        view["phase"] == "ready"
    });
    assert_eq!(view["waitingForIdle"], false);
    assert_eq!(view["canInstall"], true);
    std::thread::sleep(Duration::from_millis(500));
    assert_eq!(
        state(&updater)["phase"],
        "ready",
        "not installed on its own"
    );
    updater.stop();
    std::fs::set_permissions(&applications, std::fs::Permissions::from_mode(0o755)).unwrap();
}
