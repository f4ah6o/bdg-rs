use std::process::Command;

#[test]
fn add_license_prefers_static_manifest_dual_license_badge() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(
        temp.path().join("Cargo.toml"),
        r#"
[package]
name = "bdg-dual-license-fixture"
version = "0.1.0"
license = "MIT OR Apache-2.0"
repository = "https://github.com/f4ah6o/shuttle-rs"
"#,
    )
    .unwrap();
    std::fs::write(temp.path().join("README.md"), "# fixture\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_bdg"))
        .current_dir(temp.path())
        .args(["add", "--yes", "--only", "license", "--dry-run"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg"));
    assert!(!stdout.contains("img.shields.io/github/license"));
    assert!(output.stderr.is_empty());
}

#[test]
fn add_yes_includes_practical_rust_badges() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(
        temp.path().join("Cargo.toml"),
        r#"
[package]
name = "bdg-practical-fixture"
version = "0.1.0"
license = "MIT"
repository = "https://github.com/f4ah6o/bdg-rs"
"#,
    )
    .unwrap();
    std::fs::write(temp.path().join("README.md"), "# fixture\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_bdg"))
        .current_dir(temp.path())
        .args(["add", "--yes", "--dry-run"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("img.shields.io/crates/v/bdg-practical-fixture.svg"));
    assert!(stdout.contains("img.shields.io/crates/d/bdg-practical-fixture.svg"));
    assert!(stdout.contains("https://docs.rs/bdg-practical-fixture/badge.svg"));
    assert!(stdout.contains("img.shields.io/github/v/release/f4ah6o/bdg-rs.svg"));
    assert!(stdout.contains("img.shields.io/codecov/c/github/f4ah6o/bdg-rs.svg"));
    assert!(output.stderr.is_empty());
}

#[test]
fn add_only_coverage_filters_practical_badges() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(
        temp.path().join("Cargo.toml"),
        r#"
[package]
name = "bdg-coverage-fixture"
version = "0.1.0"
repository = "https://github.com/f4ah6o/bdg-rs"
"#,
    )
    .unwrap();
    std::fs::write(temp.path().join("README.md"), "# fixture\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_bdg"))
        .current_dir(temp.path())
        .args(["add", "--yes", "--only", "coverage", "--dry-run"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("img.shields.io/codecov/c/github/f4ah6o/bdg-rs.svg"));
    assert!(!stdout.contains("img.shields.io/crates/v/bdg-coverage-fixture.svg"));
    assert!(!stdout.contains("img.shields.io/github/v/release/f4ah6o/bdg-rs.svg"));
    assert!(output.stderr.is_empty());
}

#[test]
fn help_command_prints_usage() {
    let output = Command::new(env!("CARGO_BIN_EXE_bdg"))
        .arg("--help")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Usage:"));
    assert!(stdout.contains("bdg <COMMAND> [OPTIONS]"));
    assert!(stdout.contains("remove"));
    assert!(output.stderr.is_empty());
}

#[test]
fn version_command_prints_package_version() {
    let output = Command::new(env!("CARGO_BIN_EXE_bdg"))
        .arg("--version")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        format!("{}\n", env!("CARGO_PKG_VERSION"))
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn parse_error_exits_with_code_two() {
    let output = Command::new(env!("CARGO_BIN_EXE_bdg"))
        .args(["add", "--unknown"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("unexpected argument `--unknown`"));
    assert!(stderr.contains("Usage:"));
    assert!(output.stdout.is_empty());
}
