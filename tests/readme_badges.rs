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
fn parses_static_dual_license_kind() {
    let line = "![license](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)";
    let badge = parse_badge_line(line);
    assert_eq!(badge.kind, "license");
    assert_eq!(badge.id, "license:static");
    assert_eq!(
        badge.meta.unwrap(),
        serde_json::json!({ "license": "MIT OR Apache-2.0" })
    );
}

#[test]
fn parses_release_kind() {
    let line = "![release](https://img.shields.io/github/v/release/OWNER/REPO.svg)";
    let badge = parse_badge_line(line);
    assert_eq!(badge.kind, "github_release");
    assert_eq!(badge.id, "release:github");
    assert_eq!(
        badge.meta.unwrap(),
        serde_json::json!({ "owner": "OWNER", "repo": "REPO" })
    );
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

    let line = "![dl](https://img.shields.io/github/downloads/OWNER/REPO/total.svg)";
    let badge = parse_badge_line(line);
    assert_eq!(badge.kind, "github_downloads");
    assert_eq!(badge.id, "github_downloads:github");
}

#[test]
fn parses_coverage_kind() {
    let line = "![codecov](https://img.shields.io/codecov/c/github/OWNER/REPO.svg)";
    let badge = parse_badge_line(line);
    assert_eq!(badge.kind, "coverage");
    assert_eq!(badge.id, "coverage:codecov");
    assert_eq!(
        badge.meta.unwrap(),
        serde_json::json!({ "owner": "OWNER", "repo": "REPO" })
    );
}

#[test]
fn parses_docs_rs_badge() {
    let line = "[![docs.rs](https://docs.rs/bdg/badge.svg)](https://docs.rs/bdg)";
    let badge = parse_badge_line(line);
    assert_eq!(badge.kind, "docs");
    assert_eq!(badge.id, "docs:docsrs:bdg");
    assert_eq!(
        badge.meta.unwrap(),
        serde_json::json!({ "crate": "bdg", "provider": "docs.rs" })
    );
}

#[test]
fn parses_docs_custom_badge() {
    let line = "![docs](https://img.shields.io/badge/docs-api-blue)";
    let badge = parse_badge_line(line);
    assert_eq!(badge.kind, "docs");
    assert_eq!(badge.id, "docs:custom");
}

#[test]
fn parses_msrv_and_github_repository_badges() {
    let cases = [
        (
            "![MSRV](https://img.shields.io/crates/msrv/bdg.svg)",
            "crates_msrv",
            "crates_msrv:bdg",
        ),
        (
            "![stars](https://img.shields.io/github/stars/OWNER/REPO.svg)",
            "github_stars",
            "stars:github",
        ),
        (
            "![forks](https://img.shields.io/github/forks/OWNER/REPO.svg)",
            "github_forks",
            "forks:github",
        ),
        (
            "![issues](https://img.shields.io/github/issues/OWNER/REPO.svg)",
            "github_issues",
            "issues:github",
        ),
        (
            "![pulls](https://img.shields.io/github/issues-pr/OWNER/REPO.svg)",
            "github_pull_requests",
            "pulls:github",
        ),
        (
            "![activity](https://img.shields.io/github/last-commit/OWNER/REPO.svg)",
            "github_last_commit",
            "activity:last_commit",
        ),
    ];

    for (line, kind, id) in cases {
        let badge = parse_badge_line(line);
        assert_eq!(badge.kind, kind);
        assert_eq!(badge.id, id);
    }
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
    let lines = [
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
fn arbitrary_http_badge_is_supported_as_external() {
    let line = "![x](https://example.com/thing.svg)";
    let badge = parse_badge_line(line);
    assert_eq!(badge.kind, "external");
    assert!(badge.id.starts_with("external:"));
}

#[test]
fn id_is_stable() {
    let line = "![x](https://example.com/thing.svg)";
    let first = parse_badge_line(line).id;
    let second = parse_badge_line(line).id;
    assert_eq!(first, second);
}
