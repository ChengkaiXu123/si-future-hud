use super::*;

#[test]
fn antigravity_cli_connects_canonical_paths_persists_reopens_and_disconnects() {
    let temp = tempfile::tempdir().unwrap();
    let mut runtime = isolated(temp.path());
    let path = temp.path().join("home/profile/settings.json");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let original =
        json!({"statusLine":{"type":"command","command":"cat","padding":3},"theme":"dark"});
    fs::write(&path, serde_json::to_vec(&original).unwrap()).unwrap();
    let original_bytes = fs::read(&path).unwrap();
    assert_eq!(status(&mut runtime)["installable"], false);
    delegate(&runtime, temp.path());
    let available = status(&mut runtime);
    assert_eq!(available["installable"], true);
    assert_eq!(available["notice"], Value::Null);
    assert_eq!(
        available["defaultSettings"],
        json!(temp
            .path()
            .join("home/.gemini/antigravity-cli/settings.json"))
    );
    assert_eq!(
        fs::read(&path).unwrap(),
        original_bytes,
        "delegation alone must not install"
    );

    let connected = set_paths(&mut runtime, json!(["~/profile/settings.json"])).unwrap();
    let canonical = path.canonicalize().unwrap();
    assert_eq!(
        connected["settings"]["antigravityCliSettings"],
        json!([canonical])
    );
    assert_eq!(
        read(&temp.path().join("app-data/settings.json"))["settings"]["antigravityCliSettings"],
        json!([canonical])
    );
    assert!(read(&path)["statusLine"]["command"]
        .as_str()
        .unwrap()
        .contains("--antigravity-statusline-hook"));
    let installed_bytes = fs::read(&path).unwrap();
    drop(runtime);

    let mut reopened = isolated(temp.path());
    assert_eq!(reopened.core.antigravity_cli_settings(), [canonical]);
    assert_eq!(
        fs::read(&path).unwrap(),
        installed_bytes,
        "reopening does not rewrite CLI settings"
    );
    delegate(&reopened, temp.path());
    let disconnected = set_paths(&mut reopened, json!([])).unwrap();
    assert_eq!(
        disconnected["settings"]["antigravityCliSettings"],
        json!([])
    );
    assert_eq!(read(&path), original);
    assert_eq!(
        read(&temp.path().join("app-data/settings.json"))["settings"]["antigravityCliSettings"],
        json!([])
    );
}

#[test]
fn antigravity_cli_core_save_failure_rolls_back_connect_and_disconnect_exactly() {
    let temp = tempfile::tempdir().unwrap();
    let mut runtime = isolated(temp.path());
    delegate(&runtime, temp.path());
    runtime
        .request(
            "set_setting",
            json!({"key":"quotaDisplay","value":"remaining"}),
        )
        .unwrap();
    let preferences = temp.path().join("app-data/settings.json");
    let preferences_before = fs::read(&preferences).unwrap();
    let path = temp.path().join("home/settings.json");
    let new_path = temp.path().join("home/new/settings.json");
    let original = b"{ \"statusLine\" : {\"command\":\"cat\"}, \"theme\" : \"dark\" }\n\n";
    fs::write(&path, original).unwrap();
    let blocked_save = temp.path().join("app-data/settings.json.tmp");
    fs::create_dir(&blocked_save).unwrap();
    assert!(set_paths(&mut runtime, json!([path, new_path])).is_err());
    assert_eq!(fs::read(&path).unwrap(), original);
    assert!(!new_path.exists());
    assert_eq!(fs::read(&preferences).unwrap(), preferences_before);
    assert!(runtime.core.antigravity_cli_settings().is_empty());

    fs::remove_dir(&blocked_save).unwrap();
    set_paths(&mut runtime, json!([path])).unwrap();
    let installed = fs::read(&path).unwrap();
    let preferences_before = fs::read(&preferences).unwrap();
    let connected = runtime.core.antigravity_cli_settings();
    fs::create_dir(&blocked_save).unwrap();
    assert!(set_paths(&mut runtime, json!([])).is_err());
    assert_eq!(fs::read(&path).unwrap(), installed);
    assert_eq!(fs::read(&preferences).unwrap(), preferences_before);
    assert_eq!(runtime.core.antigravity_cli_settings(), connected);
}
