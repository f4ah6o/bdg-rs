use bdg::readme::{rewrite_marker_block_lines, BDG_BEGIN, BDG_END};
use bdg::readme_badges::parse_badge_line;
use bdg::readme_remove::remove_block_lines_by_id_kind;

fn wrap_block(lines: &[&str]) -> String {
    let mut content = String::new();
    content.push_str(BDG_BEGIN);
    content.push('\n');
    for line in lines {
        content.push_str(line);
        content.push('\n');
    }
    content.push_str(BDG_END);
    content
}

#[test]
fn remove_by_id_single() {
    let lines = vec![
        "[![CI](https://github.com/OWNER/REPO/actions/workflows/ci.yml/badge.svg)](https://github.com/OWNER/REPO/actions/workflows/ci.yml)",
        "![crate](https://img.shields.io/crates/v/foo.svg)",
    ];
    let content = wrap_block(&lines);
    let badges = lines
        .iter()
        .map(|line| parse_badge_line(line).id)
        .collect::<Vec<_>>();
    let id_to_remove = badges[0].clone();

    let outcome =
        remove_block_lines_by_id_kind(&content, &[id_to_remove.clone()], &[], false).unwrap();
    let updated = rewrite_marker_block_lines(&content, &outcome.remaining).unwrap();
    assert!(!updated.contains(&id_to_remove));
    assert!(updated.contains("crates/v/foo"));
}

#[test]
fn remove_by_kind_multiple() {
    let lines = vec![
        "![npm](https://img.shields.io/npm/v/pkg.svg)",
        "![npm](https://img.shields.io/npm/dt/pkg.svg)",
        "![crate](https://img.shields.io/crates/v/foo.svg)",
    ];
    let content = wrap_block(&lines);
    let outcome =
        remove_block_lines_by_id_kind(&content, &[], &["npm_downloads".to_string()], false)
            .unwrap();
    let updated = rewrite_marker_block_lines(&content, &outcome.remaining).unwrap();
    assert!(!updated.contains("npm/dt"));
    assert!(updated.contains("crates/v/foo"));
}

#[test]
fn unknown_can_be_removed_by_id() {
    let line = "[![label][imgref]](linkref)";
    let badge = parse_badge_line(line);
    let content = wrap_block(&[line]);
    let outcome = remove_block_lines_by_id_kind(&content, &[badge.id], &[], false).unwrap();
    let updated = rewrite_marker_block_lines(&content, &outcome.remaining).unwrap();
    assert!(updated.contains(BDG_BEGIN));
    assert!(updated.contains(BDG_END));
    assert!(!updated.contains(&badge.raw));
}

#[test]
fn strict_mode_errors_when_id_missing() {
    let lines = vec!["![crate](https://img.shields.io/crates/v/foo.svg)"];
    let content = wrap_block(&lines);
    let result = remove_block_lines_by_id_kind(&content, &["ci:ci.yml".to_string()], &[], true);
    assert!(result.is_err());
}

#[test]
fn missing_ids_are_reported() {
    let lines = vec!["![crate](https://img.shields.io/crates/v/foo.svg)"];
    let content = wrap_block(&lines);
    let outcome =
        remove_block_lines_by_id_kind(&content, &["ci:ci.yml".to_string()], &[], false).unwrap();
    assert_eq!(outcome.missing_ids, vec!["ci:ci.yml".to_string()]);
}

#[test]
fn does_not_remove_inside_code_fence() {
    let lines = vec![
        "```md",
        "![crate](https://img.shields.io/crates/v/foo.svg)",
        "```",
        "![crate](https://img.shields.io/crates/v/bar.svg)",
    ];
    let content = wrap_block(&lines);
    let outcome =
        remove_block_lines_by_id_kind(&content, &[], &["crates_version".to_string()], false)
            .unwrap();
    let updated = rewrite_marker_block_lines(&content, &outcome.remaining).unwrap();
    assert!(updated.contains("crates/v/foo"));
    assert!(!updated.contains("crates/v/bar"));
}
