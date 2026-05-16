use bdg::badges::badge_for_workflow;
use bdg::workflows::detect_workflows;
use std::fs;

#[test]
fn detects_yaml_and_yml_workflows() {
    let temp = tempfile::tempdir().unwrap();
    let workflows_dir = temp.path().join(".github/workflows");
    fs::create_dir_all(&workflows_dir).unwrap();
    fs::write(workflows_dir.join("ci.yml"), "name: ci\n").unwrap();
    fs::write(workflows_dir.join("release.yaml"), "name: release\n").unwrap();

    let workflows = detect_workflows(temp.path());
    let files = workflows
        .iter()
        .map(|workflow| workflow.file.as_str())
        .collect::<Vec<_>>();

    assert_eq!(files, vec!["ci.yml", "release.yaml"]);
}

#[test]
fn workflow_badge_uses_actual_workflow_file_extension() {
    let badge = badge_for_workflow("f4ah6o", "codegraph", "rust.yml");

    assert_eq!(
        badge.image_url,
        "https://github.com/f4ah6o/codegraph/actions/workflows/rust.yml/badge.svg"
    );
    assert_eq!(
        badge.link_url.as_deref(),
        Some("https://github.com/f4ah6o/codegraph/actions/workflows/rust.yml")
    );
}
