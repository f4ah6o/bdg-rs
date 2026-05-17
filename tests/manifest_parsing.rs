use bdg::core::detect_manifests;
use bdg::manifest::{
    RepositoryField, read_cargo_toml, read_moon_mod, read_package_json, read_resolved_cargo_package,
};
use std::fs;
use std::path::Path;

fn fixture(path: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(path)
}

#[test]
fn parses_package_json() {
    let path = fixture("package.json");
    let pkg = read_package_json(&path).unwrap();
    assert_eq!(pkg.name.as_deref(), Some("bdg-fixture"));
    assert_eq!(pkg.version.as_deref(), Some("1.2.3"));
    assert_eq!(pkg.license.as_deref(), Some("MIT"));
    match pkg.repository {
        Some(RepositoryField::String(value)) => {
            assert_eq!(value, "https://github.com/example/bdg-fixture")
        }
        _ => panic!("unexpected repository field"),
    }
}

#[test]
fn parses_package_json_private_flag() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("package.json");
    fs::write(
        &path,
        r#"{ "name": "private-root", "version": "1.0.0", "private": true }"#,
    )
    .unwrap();

    let pkg = read_package_json(&path).unwrap();

    assert_eq!(pkg.name.as_deref(), Some("private-root"));
    assert_eq!(pkg.private, Some(true));
}

#[test]
fn parses_moon_mod_json() {
    let path = fixture("moon.mod.json");
    let module = read_moon_mod(&path).unwrap();
    assert_eq!(module.name.as_deref(), Some("moon-fixture"));
    assert_eq!(module.version.as_deref(), Some("0.4.0"));
}

#[test]
fn parses_cargo_toml() {
    let path = fixture("Cargo.toml");
    let package = read_resolved_cargo_package(&path)
        .unwrap()
        .expect("package missing");
    assert_eq!(package.name.as_deref(), Some("bdg-fixture"));
    assert_eq!(package.version.as_deref(), Some("0.5.0"));
    assert_eq!(package.license.as_deref(), Some("Apache-2.0"));
    assert_eq!(
        package.repository.as_deref(),
        Some("https://github.com/example/bdg-fixture")
    );
}

#[test]
fn chooses_closest_manifest() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures");
    let current = root.join("nested");
    let manifests = detect_manifests(&root, &current, 3).unwrap();
    let package = manifests.package_json.as_ref().unwrap();
    assert!(package.to_string_lossy().contains("nested"));
    assert!(manifests.cargo_toml.is_some());
}

#[test]
fn ignores_fixtures_manifests() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let current = root;
    let manifests = detect_manifests(root, current, 5).unwrap();
    assert!(
        !manifests
            .moon_mod_all
            .iter()
            .any(|path| path.to_string_lossy().contains("tests/fixtures"))
    );
}

#[test]
fn chooses_workspace_member_manifest_from_workspace_root() {
    let temp = tempfile::tempdir().unwrap();
    write_workspace_fixture(temp.path());

    let manifests = detect_manifests(temp.path(), temp.path(), 3).unwrap();
    let cargo_toml = manifests.cargo_toml.as_ref().expect("Cargo.toml missing");

    assert!(cargo_toml.ends_with("crates/codegraph/Cargo.toml"));
}

#[test]
fn resolves_workspace_inherited_cargo_package_fields() {
    let temp = tempfile::tempdir().unwrap();
    write_workspace_fixture(temp.path());
    let manifest_path = temp.path().join("crates/codegraph/Cargo.toml");

    let package = read_resolved_cargo_package(&manifest_path)
        .unwrap()
        .expect("package missing");

    assert_eq!(package.name.as_deref(), Some("cgz"));
    assert_eq!(package.license.as_deref(), Some("MIT"));
    assert_eq!(
        package.repository.as_deref(),
        Some("https://github.com/f4ah6o/codegraph")
    );
}

#[test]
fn raw_workspace_inherited_cargo_toml_parses() {
    let temp = tempfile::tempdir().unwrap();
    write_workspace_fixture(temp.path());
    let manifest_path = temp.path().join("crates/codegraph/Cargo.toml");

    let manifest = read_cargo_toml(&manifest_path).unwrap();

    assert!(manifest.package.is_some());
}

fn write_workspace_fixture(root: &Path) {
    fs::create_dir_all(root.join("crates/codegraph")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        r#"
[workspace]
members = ["crates/codegraph"]

[workspace.package]
license = "MIT"
repository = "https://github.com/f4ah6o/codegraph"
"#,
    )
    .unwrap();
    fs::write(
        root.join("crates/codegraph/Cargo.toml"),
        r#"
[package]
name = "cgz"
version = "0.1.0"
license.workspace = true
repository.workspace = true
"#,
    )
    .unwrap();
}
