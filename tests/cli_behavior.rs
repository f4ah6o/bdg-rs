use std::process::Command;

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
