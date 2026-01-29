use bdg::core::detect_manifests;
use bdg::manifest::{read_cargo_toml, read_moon_mod, read_package_json, RepositoryField};
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
fn parses_moon_mod_json() {
    let path = fixture("moon.mod.json");
    let module = read_moon_mod(&path).unwrap();
    assert_eq!(module.name.as_deref(), Some("moon-fixture"));
    assert_eq!(module.version.as_deref(), Some("0.4.0"));
}

#[test]
fn parses_cargo_toml() {
    let path = fixture("Cargo.toml");
    let manifest = read_cargo_toml(&path).unwrap();
    let package = manifest.package.expect("package missing");
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
    assert!(!manifests
        .moon_mod_all
        .iter()
        .any(|path| path.to_string_lossy().contains("tests/fixtures")));
}
