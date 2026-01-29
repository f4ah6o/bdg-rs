use bdg::config::load_config;
use std::fs;

fn write_config(path: &std::path::Path, allow_yy: bool) {
    let content = format!("[version]\nallow_yy_calver = {}\n", allow_yy);
    fs::write(path, content).unwrap();
}

#[test]
fn prefers_nearest_config() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path().join("repo");
    let cwd = repo.join("packages").join("a");
    fs::create_dir_all(&cwd).unwrap();
    fs::create_dir_all(repo.join(".git")).unwrap();
    write_config(&repo.join(".bdg.toml"), false);
    write_config(&cwd.join(".bdg.toml"), true);

    let config = load_config(&cwd, &repo).unwrap();
    assert!(config.version.allow_yy_calver);
}

#[test]
fn falls_back_to_repo_root() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path().join("repo");
    let cwd = repo.join("packages").join("a");
    fs::create_dir_all(&cwd).unwrap();
    fs::create_dir_all(repo.join(".git")).unwrap();
    write_config(&repo.join(".bdg.toml"), true);

    let config = load_config(&cwd, &repo).unwrap();
    assert!(config.version.allow_yy_calver);
}

#[test]
fn does_not_escape_git_root() {
    let dir = tempfile::tempdir().unwrap();
    let outer = dir.path().join("outer");
    let repo = outer.join("repo");
    let cwd = repo.join("packages").join("a");
    fs::create_dir_all(&cwd).unwrap();
    fs::create_dir_all(repo.join(".git")).unwrap();
    fs::create_dir_all(&outer).unwrap();
    write_config(&outer.join(".bdg.toml"), true);

    let config = load_config(&cwd, &repo).unwrap();
    assert!(!config.version.allow_yy_calver);
}
