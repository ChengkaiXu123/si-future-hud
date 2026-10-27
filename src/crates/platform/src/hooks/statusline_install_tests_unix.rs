use super::*;
use std::{
    collections::HashMap,
    fs,
    io::Write,
    os::unix::fs::{symlink, PermissionsExt},
    process::{Command, Stdio},
};

fn context(home: &Path) -> InstallContext {
    let executable = home.join("fixed app's owner");
    fs::write(&executable, "#!/bin/sh\ncat > \"$2/callback-input\"\nprintf '%s' \"$1\" > \"$2/callback-flag\"\nprintf '%s' \"$3\" > \"$2/callback-settings\"\nprintf 'discarded callback output'\nprintf 'discarded callback error' >&2\nexit 19\n").unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    InstallContext {
        home: home.to_owned(),
        environment: HashMap::new(),
        canonical_executable: Some(executable.clone()),
        executable,
    }
}

fn read(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

fn write(path: &Path, value: &Value) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
}

#[test]
fn wraps_a_foreign_renderer_without_changing_input_output_options_or_other_settings() {
    let temp = tempfile::tempdir().unwrap();
    let context = context(temp.path());
    let data = temp.path().join("callback data's folder");
    fs::create_dir(&data).unwrap();
    let path = temp.path().join("settings with 'quotes'.json");
    let original = json!({
        "statusLine":{"type":"command","command":"printf 'original:'; cat; exit 7", "padding":4, "stack_with_default":false, "custom":{"unchanged":true}},
        "private":{"apiKey":"fixture-private-value"}, "enabled":false,
    });
    write(&path, &original);
    fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
    let changed = set_accounts(Some(&context), &data, &[], std::slice::from_ref(&path)).unwrap();
    assert_eq!(changed.settings(), [path.canonicalize().unwrap()]);
    let installed = read(&path);
    assert_eq!(installed["statusLine"]["padding"], 4);
    assert_eq!(installed["statusLine"]["stack_with_default"], false);
    assert_eq!(
        installed["statusLine"]["custom"],
        original["statusLine"]["custom"]
    );
    assert_eq!(installed["private"], original["private"]);
    assert_eq!(installed["enabled"], original["enabled"]);
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o640
    );

    let mut process = Command::new("/bin/sh")
        .args(["-c", installed["statusLine"]["command"].as_str().unwrap()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let payload = b"{\"literal\":\"$() `quoted` ' \\\"\"}\n\n";
    process.stdin.take().unwrap().write_all(payload).unwrap();
    let output = process.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(7));
    assert_eq!(output.stdout, [b"original:".as_slice(), payload].concat());
    assert!(output.stderr.is_empty());
    assert_eq!(fs::read(data.join("callback-input")).unwrap(), payload);
    assert_eq!(
        fs::read_to_string(data.join("callback-flag")).unwrap(),
        FLAG
    );
    assert_eq!(
        fs::read_to_string(data.join("callback-settings")).unwrap(),
        path.canonicalize().unwrap().to_str().unwrap()
    );

    set_accounts(Some(&context), &data, changed.settings(), &[]).unwrap();
    assert_eq!(
        read(&path),
        original,
        "uninstall restores the exact renderer object and leaves other settings"
    );
}

#[test]
fn a_new_statusline_keeps_the_default_and_new_files_are_removed_on_uninstall() {
    let temp = tempfile::tempdir().unwrap();
    let context = context(temp.path());
    let path = temp.path().join("profile/settings.json");
    let changed = set_accounts(
        Some(&context),
        temp.path(),
        &[],
        std::slice::from_ref(&path),
    )
    .unwrap();
    assert_eq!(read(&path)["statusLine"]["stack_with_default"], true);
    let command = read(&path)["statusLine"]["command"]
        .as_str()
        .unwrap()
        .to_owned();
    let mut child = Command::new("/bin/sh")
        .args(["-c", &command])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"{\"usage\":1}\n")
        .unwrap();
    let result = child.wait_with_output().unwrap();
    assert!(result.status.success());
    assert!(result.stdout.is_empty() && result.stderr.is_empty());
    set_accounts(Some(&context), temp.path(), changed.settings(), &[]).unwrap();
    assert!(!path.exists());
}

#[test]
fn a_disabled_renderer_stays_silent_while_collection_runs_and_is_restored_on_disconnect() {
    let temp = tempfile::tempdir().unwrap();
    let context = context(temp.path());
    for (index, line) in [
        json!({"enabled":false,"command":"printf 'disabled renderer must not run'","padding":4,"stack_with_default":false}),
        json!({"enabled":false,"padding":2}),
    ].into_iter().enumerate() {
        let path = temp.path().join(format!("settings-{index}.json"));
        let original = json!({"statusLine":line,"theme":"dark"});
        write(&path, &original);
        let changed = set_accounts(Some(&context), temp.path(), &[], std::slice::from_ref(&path)).unwrap();
        let installed = read(&path);
        assert_eq!(installed["statusLine"]["enabled"], true);
        assert_eq!(installed["statusLine"]["stack_with_default"], true);
        assert_eq!(installed["statusLine"]["padding"], original["statusLine"]["padding"]);
        let mut child = Command::new("/bin/sh")
            .args(["-c", installed["statusLine"]["command"].as_str().unwrap()])
            .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())
            .spawn().unwrap();
        let payload = b"{\"usage\":1}\n";
        child.stdin.take().unwrap().write_all(payload).unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success());
        assert!(output.stdout.is_empty() && output.stderr.is_empty());
        assert_eq!(fs::read(temp.path().join("callback-input")).unwrap(), payload);
        set_accounts(Some(&context), temp.path(), changed.settings(), &[]).unwrap();
        assert_eq!(read(&path), original);
    }
}

#[test]
fn later_other_settings_survive_uninstall_of_a_new_file() {
    let temp = tempfile::tempdir().unwrap();
    let context = context(temp.path());
    let path = temp.path().join("settings.json");
    let changed = set_accounts(
        Some(&context),
        temp.path(),
        &[],
        std::slice::from_ref(&path),
    )
    .unwrap();
    let mut edited = read(&path);
    edited["theme"] = json!("dark");
    write(&path, &edited);
    set_accounts(Some(&context), temp.path(), changed.settings(), &[]).unwrap();
    assert_eq!(read(&path), json!({"theme":"dark"}));
}

#[test]
fn repeated_install_is_idempotent_and_rollback_restores_exact_bytes() {
    let temp = tempfile::tempdir().unwrap();
    let context = context(temp.path());
    let path = temp.path().join("settings.json");
    let bytes = b"{ \"theme\" : \"dark\", \"statusLine\": {\"command\":\"cat\"} }\n\n";
    fs::write(&path, bytes).unwrap();
    let first = set_accounts(
        Some(&context),
        temp.path(),
        &[],
        std::slice::from_ref(&path),
    )
    .unwrap();
    let installed = fs::read(&path).unwrap();
    let repeat = set_accounts(
        Some(&context),
        temp.path(),
        first.settings(),
        first.settings(),
    )
    .unwrap();
    assert!(repeat.writes.is_empty());
    assert_eq!(fs::read(&path).unwrap(), installed);
    first.rollback().unwrap();
    assert_eq!(fs::read(&path).unwrap(), bytes);
}

#[test]
fn refuses_modified_renderers_foreign_metadata_and_another_owner() {
    let temp = tempfile::tempdir().unwrap();
    let context = context(temp.path());
    let path = temp.path().join("settings.json");
    set_accounts(
        Some(&context),
        temp.path(),
        &[],
        std::slice::from_ref(&path),
    )
    .unwrap();
    let installed = read(&path);
    for pointer in [
        "/statusLine/command",
        "/statusLine/stack_with_default",
        "/_agentHudAntigravityStatusLine/createdSettings",
    ] {
        let mut changed = installed.clone();
        *changed.pointer_mut(pointer).unwrap() = json!("foreign private renderer");
        write(&path, &changed);
        let before = fs::read(&path).unwrap();
        let error = set_accounts(
            Some(&context),
            temp.path(),
            std::slice::from_ref(&path),
            &[],
        )
        .err()
        .unwrap();
        assert!(!error.contains("foreign private renderer"));
        assert_eq!(fs::read(&path).unwrap(), before);
    }
    write(&path, &json!({METADATA:{"foreign":"do not overwrite"}}));
    assert!(set_accounts(
        Some(&context),
        temp.path(),
        &[],
        std::slice::from_ref(&path)
    )
    .is_err());
    write(&path, &installed);
    let alternate_data = temp.path().join("another-app");
    assert!(set_accounts(
        Some(&context),
        &alternate_data,
        std::slice::from_ref(&path),
        &[]
    )
    .is_err());
    let mut changed = installed;
    changed.as_object_mut().unwrap().remove(METADATA);
    write(&path, &changed);
    assert!(set_accounts(Some(&context), temp.path(), &[], &[path]).is_err());
}

#[test]
fn partial_failure_rolls_back_prior_files_without_overwriting_external_edits() {
    let temp = tempfile::tempdir().unwrap();
    let context = context(temp.path());
    let first = temp.path().join("first.json");
    let second = temp.path().join("second.json");
    let original = b"{ \"theme\" : \"light\" }\n";
    fs::write(&first, original).unwrap();
    fs::write(&second, original).unwrap();
    let prepared = plan(&context, temp.path(), &[], &[first.clone(), second.clone()]).unwrap();
    let external = b"{\"external\":true}\n";
    fs::write(&second, external).unwrap();
    assert!(apply(prepared).is_err());
    assert_eq!(fs::read(&first).unwrap(), original);
    assert_eq!(fs::read(&second).unwrap(), external);
    let created = temp.path().join("created.json");
    let prepared = plan(
        &context,
        temp.path(),
        &[],
        &[created.clone(), second.clone()],
    )
    .unwrap();
    fs::write(&second, original).unwrap();
    assert!(apply(prepared).is_err());
    assert!(!created.exists());
}

#[test]
fn core_save_failure_can_undo_uninstall_but_never_overwrites_a_later_edit() {
    let temp = tempfile::tempdir().unwrap();
    let context = context(temp.path());
    let path = temp.path().join("settings.json");
    let installed = set_accounts(
        Some(&context),
        temp.path(),
        &[],
        std::slice::from_ref(&path),
    )
    .unwrap();
    let bytes = fs::read(&path).unwrap();
    let removed = set_accounts(Some(&context), temp.path(), installed.settings(), &[]).unwrap();
    assert!(!path.exists());
    removed.rollback().unwrap();
    assert_eq!(fs::read(&path).unwrap(), bytes);
    let changed = b"{\"userEdited\":true}";
    fs::write(&path, changed).unwrap();
    assert!(installed.rollback().is_err());
    assert_eq!(fs::read(&path).unwrap(), changed);
}

#[test]
fn canonical_aliases_and_tilde_share_one_scope_but_duplicate_selections_are_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let context = context(temp.path());
    let real = temp.path().join("real");
    fs::create_dir(&real).unwrap();
    let alias = temp.path().join("alias");
    symlink(&real, &alias).unwrap();
    let path = real.join("settings.json");
    assert!(set_accounts(
        Some(&context),
        temp.path(),
        &[],
        &[path.clone(), alias.join("settings.json")]
    )
    .is_err());
    assert!(!path.exists());
    let installed = set_accounts(
        Some(&context),
        temp.path(),
        &[],
        &[PathBuf::from("~/alias/settings.json")],
    )
    .unwrap();
    assert_eq!(installed.settings(), [path.canonicalize().unwrap()]);
    set_accounts(Some(&context), temp.path(), installed.settings(), &[]).unwrap();
    assert!(!path.exists());
}

#[test]
fn a_retargeted_symlink_is_not_written_and_size_or_json_errors_leave_all_files_alone() {
    let temp = tempfile::tempdir().unwrap();
    let context = context(temp.path());
    let first = temp.path().join("one.json");
    let second = temp.path().join("two.json");
    write(&first, &json!({"first":true}));
    write(&second, &json!({"second":true}));
    let alias = temp.path().join("alias.json");
    symlink(&first, &alias).unwrap();
    let prepared = plan(&context, temp.path(), &[], std::slice::from_ref(&alias)).unwrap();
    fs::remove_file(&alias).unwrap();
    symlink(&second, &alias).unwrap();
    assert!(apply(prepared).is_err());
    assert_eq!(read(&first), json!({"first":true}));
    assert_eq!(read(&second), json!({"second":true}));
    for invalid in [b"not json".as_slice(), b"[]".as_slice()] {
        fs::write(&second, invalid).unwrap();
        assert!(set_accounts(
            Some(&context),
            temp.path(),
            &[],
            &[first.clone(), second.clone()]
        )
        .is_err());
        assert_eq!(read(&first), json!({"first":true}));
    }
    fs::File::create(&second)
        .unwrap()
        .set_len(16 * 1024 * 1024 + 1)
        .unwrap();
    assert!(set_accounts(Some(&context), temp.path(), &[], &[second]).is_err());
}

#[test]
fn status_and_install_require_the_current_fixed_owner() {
    let temp = tempfile::tempdir().unwrap();
    let mut context = context(temp.path());
    let path = temp.path().join("settings.json");
    assert_eq!(status(Some(&context))["installable"], true);
    assert_eq!(
        status(Some(&context))["defaultSettings"],
        json!(temp.path().join(".gemini/antigravity-cli/settings.json"))
    );
    assert_eq!(status(None)["installable"], false);
    context.canonical_executable = None;
    assert_eq!(status(Some(&context))["installable"], false);
    assert!(set_accounts(
        Some(&context),
        temp.path(),
        &[],
        std::slice::from_ref(&path)
    )
    .is_err());
    assert!(!path.exists());
    assert!(set_accounts(None, temp.path(), &[], &[path]).is_err());
}
