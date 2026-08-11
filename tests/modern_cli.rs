use std::process::Command;

fn write_moon_project(root: &std::path::Path) {
    std::fs::write(
        root.join("moon.mod.json"),
        r#"{"name":"example/demo","version":"1.2.3"}"#,
    )
    .unwrap();
    std::fs::write(root.join("README.md"), "# demo\n").unwrap();
}

#[test]
fn sync_check_is_ci_friendly_and_idempotent() {
    let temp = tempfile::tempdir().unwrap();
    write_moon_project(temp.path());

    let pending = Command::new(env!("CARGO_BIN_EXE_bdg"))
        .current_dir(temp.path())
        .args(["sync", "--only", "version", "--check"])
        .output()
        .unwrap();
    assert_eq!(pending.status.code(), Some(2));
    assert!(
        String::from_utf8(pending.stdout)
            .unwrap()
            .contains("moonbit")
    );

    let apply = Command::new(env!("CARGO_BIN_EXE_bdg"))
        .current_dir(temp.path())
        .args(["sync", "--only", "version"])
        .output()
        .unwrap();
    assert!(apply.status.success());

    let clean = Command::new(env!("CARGO_BIN_EXE_bdg"))
        .current_dir(temp.path())
        .args(["sync", "--only", "version", "--check"])
        .output()
        .unwrap();
    assert!(clean.status.success());
    assert!(clean.stdout.is_empty());
}

#[test]
fn check_reports_marker_problems_as_json() {
    let temp = tempfile::tempdir().unwrap();
    write_moon_project(temp.path());

    let output = Command::new(env!("CARGO_BIN_EXE_bdg"))
        .args(["-C", temp.path().to_str().unwrap(), "check", "--json"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["schema"], "bdg.check/v1");
    assert_eq!(value["ok"], false);
    assert_eq!(value["issues"][0]["code"], "MARKER_MISSING");
}

#[test]
fn list_does_not_pretend_missing_markers_exist() {
    let temp = tempfile::tempdir().unwrap();
    write_moon_project(temp.path());

    let output = Command::new(env!("CARGO_BIN_EXE_bdg"))
        .current_dir(temp.path())
        .args(["list", "--json"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["readme"]["markers"]["present"], false);
    assert_eq!(value["readme"]["markers"]["count"], 0);
}
