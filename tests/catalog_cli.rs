use std::process::Command;

fn fixture() -> tempfile::TempDir {
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(
        temp.path().join("Cargo.toml"),
        r#"
[package]
name = "catalog-fixture"
version = "0.1.0"
repository = "https://github.com/example/catalog-fixture"
"#,
    )
    .unwrap();
    std::fs::write(temp.path().join("README.md"), "# fixture\n").unwrap();
    temp
}

#[test]
fn catalog_search_lists_builtin_entries() {
    let temp = fixture();
    let output = Command::new(env!("CARGO_BIN_EXE_bdg"))
        .current_dir(temp.path())
        .args(["catalog", "search", "contributors", "--json"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let payload: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(payload["schema"], "bdg.catalog.search/v1");
    assert!(
        payload["results"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["id"] == "github-contributors")
    );
}

#[test]
fn catalog_add_expands_project_placeholders_without_network() {
    let temp = fixture();
    let source = temp.path().join("extra.toml");
    std::fs::write(
        &source,
        r#"
schema = "bdg.catalog/v1"
[[badge]]
id = "custom-repo"
kind = "custom"
label = "repo"
image = "https://img.shields.io/badge/repo-{repo}-blue.svg"
link = "https://github.com/{owner}/{repo}"
requires = ["owner", "repo"]
"#,
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_bdg"))
        .current_dir(temp.path())
        .args([
            "catalog",
            "add",
            "custom-repo",
            "--source",
            source.to_str().unwrap(),
            "--set",
            "owner=example",
            "--set",
            "repo=catalog-fixture",
            "--dry-run",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("img.shields.io/badge/repo-catalog-fixture-blue.svg"));
    assert!(stdout.contains("https://github.com/example/catalog-fixture"));
}

#[test]
fn catalog_add_accepts_arbitrary_external_badge_and_is_idempotent() {
    let temp = fixture();
    let source = temp.path().join("external.json");
    std::fs::write(
        &source,
        r#"{
          "schema": "bdg.catalog/v1",
          "badges": [{
            "id": "external-status",
            "kind": "status",
            "label": "status",
            "image": "https://example.com/status.svg",
            "link": "https://example.com/status"
          }]
        }"#,
    )
    .unwrap();

    let first = Command::new(env!("CARGO_BIN_EXE_bdg"))
        .current_dir(temp.path())
        .args([
            "catalog",
            "add",
            "external-status",
            "--source",
            source.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(first.status.success());

    let second = Command::new(env!("CARGO_BIN_EXE_bdg"))
        .current_dir(temp.path())
        .args([
            "catalog",
            "add",
            "external-status",
            "--source",
            source.to_str().unwrap(),
            "--dry-run",
        ])
        .output()
        .unwrap();
    assert!(second.status.success());

    let readme = std::fs::read_to_string(temp.path().join("README.md")).unwrap();
    assert!(readme.contains("https://example.com/status.svg"));
}

#[test]
fn catalog_sources_can_be_persisted_in_bdg_config() {
    let temp = fixture();
    let source = temp.path().join("team-catalog.toml");
    std::fs::write(
        &source,
        r#"
schema = "bdg.catalog/v1"
[[badge]]
id = "team-status"
kind = "team"
label = "team status"
image = "https://example.com/team-status.svg"
tags = ["team", "internal"]
"#,
    )
    .unwrap();
    std::fs::write(
        temp.path().join(".bdg.toml"),
        format!(
            "[catalog]\nsources = [{}]\n",
            serde_json::to_string(source.to_str().unwrap()).unwrap()
        ),
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_bdg"))
        .current_dir(temp.path())
        .args(["catalog", "search", "team-status", "--json"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let payload: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(payload["results"][0]["id"], "team-status");
    assert_eq!(
        payload["results"][0]["source"],
        source.to_string_lossy().as_ref()
    );
}

#[test]
fn catalog_add_url_adds_one_off_external_badge() {
    let temp = fixture();
    let output = Command::new(env!("CARGO_BIN_EXE_bdg"))
        .current_dir(temp.path())
        .args([
            "catalog",
            "add-url",
            "https://example.com/health.svg",
            "--label",
            "health",
            "--link",
            "https://example.com/health",
            "--dry-run",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("[![health](https://example.com/health.svg)](https://example.com/health)")
    );
}
