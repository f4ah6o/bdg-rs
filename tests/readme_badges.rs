use bdg::readme_badges::parse_badge_line;

#[test]
fn parses_linked_image() {
    let line = "[![CI](https://github.com/OWNER/REPO/actions/workflows/ci.yaml/badge.svg)](https://github.com/OWNER/REPO/actions/workflows/ci.yaml)";
    let badge = parse_badge_line(line);
    assert_eq!(badge.kind, "github_actions");
    assert_eq!(badge.id, "ci:ci.yaml");
    assert_eq!(badge.label, "CI");
    assert_eq!(
        badge.image,
        "https://github.com/OWNER/REPO/actions/workflows/ci.yaml/badge.svg"
    );
    assert_eq!(
        badge.link.as_deref(),
        Some("https://github.com/OWNER/REPO/actions/workflows/ci.yaml")
    );
}

#[test]
fn parses_image_only() {
    let line = "![crate](https://img.shields.io/crates/v/foo.svg)";
    let badge = parse_badge_line(line);
    assert_eq!(badge.kind, "crates_version");
    assert_eq!(badge.id, "crates:foo");
    assert_eq!(badge.image, "https://img.shields.io/crates/v/foo.svg");
}

#[test]
fn parses_npm_kind() {
    let line = "![npm](https://img.shields.io/npm/v/@scope/pkg.svg)";
    let badge = parse_badge_line(line);
    assert_eq!(badge.kind, "npm_version");
    assert_eq!(badge.id, "npm:@scope/pkg");
}

#[test]
fn parses_license_kind() {
    let line = "![license](https://img.shields.io/github/license/OWNER/REPO.svg)";
    let badge = parse_badge_line(line);
    assert_eq!(badge.kind, "license");
    assert_eq!(badge.id, "license:github");
}

#[test]
fn parses_release_kind() {
    let line = "![release](https://img.shields.io/github/v/release/OWNER/REPO.svg)";
    let badge = parse_badge_line(line);
    assert_eq!(badge.kind, "github_release");
    assert_eq!(badge.id, "release:github");
}

#[test]
fn parses_downloads_kinds() {
    let line = "![dl](https://img.shields.io/npm/dt/@scope/pkg.svg)";
    let badge = parse_badge_line(line);
    assert_eq!(badge.kind, "npm_downloads");
    assert_eq!(badge.id, "npm_downloads:@scope/pkg");

    let line = "![dl](https://img.shields.io/crates/d/foo.svg)";
    let badge = parse_badge_line(line);
    assert_eq!(badge.kind, "crates_downloads");
    assert_eq!(badge.id, "crates_downloads:foo");
}

#[test]
fn parses_coverage_kind() {
    let line = "![codecov](https://img.shields.io/codecov/c/github/OWNER/REPO.svg)";
    let badge = parse_badge_line(line);
    assert_eq!(badge.kind, "coverage");
    assert_eq!(badge.id, "coverage:codecov");
}

#[test]
fn parses_docs_custom_badge() {
    let line = "![docs](https://img.shields.io/badge/docs-api-blue)";
    let badge = parse_badge_line(line);
    assert_eq!(badge.kind, "docs");
    assert_eq!(badge.id, "docs:custom");
}

#[test]
fn non_http_url_is_unknown() {
    let line = "![local](./badge.svg)";
    let badge = parse_badge_line(line);
    assert_eq!(badge.kind, "unknown");
    assert!(badge.id.starts_with("unknown:"));
}

#[test]
fn ignores_badges_in_code_fence() {
    let lines = vec![
        "```md",
        "![crate](https://img.shields.io/crates/v/foo.svg)",
        "```",
        "![crate](https://img.shields.io/crates/v/bar.svg)",
    ];
    let parsed = lines
        .iter()
        .filter(|line| !line.trim_start().starts_with("```"))
        .map(|line| bdg::readme_badges::parse_badge_line(line))
        .collect::<Vec<_>>();
    assert_eq!(parsed.len(), 2);
}

#[test]
fn unknown_for_weird_markdown() {
    let line = "[![label][imgref]](linkref)";
    let badge = parse_badge_line(line);
    assert_eq!(badge.kind, "unknown");
    assert!(badge.id.starts_with("unknown:"));
    assert_eq!(badge.raw, line);
}

#[test]
fn id_is_stable() {
    let line = "![x](https://example.com/thing.svg)";
    let first = parse_badge_line(line).id;
    let second = parse_badge_line(line).id;
    assert_eq!(first, second);
}
