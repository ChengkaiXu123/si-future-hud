use super::*;

#[test]
fn antigravity_cli_unsupported_host_reports_unavailable_without_mutating_settings() {
    let temp = tempfile::tempdir().unwrap();
    let mut runtime = isolated(temp.path());
    delegate(&runtime, temp.path());
    let unavailable = status(&mut runtime);
    assert_eq!(unavailable["installable"], false);
    let notice = unavailable["notice"].as_str().unwrap();
    assert!(notice.contains("unavailable on this system") || notice.contains("此系统暂不支持"));
    let path = temp.path().join("home/settings.json");
    let original = b"{ \"theme\" : \"dark\" }\n";
    fs::write(&path, original).unwrap();
    assert_eq!(set_paths(&mut runtime, json!([path])).unwrap_err(), notice);
    assert_eq!(fs::read(&path).unwrap(), original);
    assert!(runtime.core.antigravity_cli_settings().is_empty());
    assert!(!temp.path().join("app-data/settings.json").exists());
    assert_eq!(read(&path), json!({"theme":"dark"}));
}
