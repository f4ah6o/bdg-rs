use bdg::readme::{
    ensure_marker_block, extract_managed_block, remove_marker_block, rewrite_marker_block,
};
use std::fs;
use tempfile::tempdir;

#[test]
fn rewrite_is_idempotent() {
    let input = "# Title\n<!-- bdg:begin -->\n![a](a)\n<!-- bdg:end -->\nBody";
    let first = rewrite_marker_block(input, &["![a](a)".to_string()]).unwrap();
    let second = rewrite_marker_block(&first, &["![a](a)".to_string()]).unwrap();
    assert_eq!(first, second);
}

#[test]
fn rewrite_does_not_change_outside_block() {
    let input = "# Title\nIntro\n<!-- bdg:begin -->\n![a](a)\n<!-- bdg:end -->\nFooter";
    let updated = rewrite_marker_block(input, &["![b](b)".to_string()]).unwrap();
    assert!(updated.contains("Intro"));
    assert!(updated.contains("Footer"));
    assert!(updated.contains("![b](b)"));
}

#[test]
fn insert_marker_under_h1() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("README.md");
    fs::write(&path, "# Title\nIntro").unwrap();
    let content = ensure_marker_block(&path).unwrap();
    let expected = "# Title\n<!-- bdg:begin -->\n<!-- bdg:end -->\nIntro";
    assert_eq!(content, expected);
}

#[test]
fn insert_marker_at_top_when_no_h1() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("README.md");
    fs::write(&path, "Intro\nLine").unwrap();
    let content = ensure_marker_block(&path).unwrap();
    let expected = "<!-- bdg:begin -->\n<!-- bdg:end -->\nIntro\nLine";
    assert_eq!(content, expected);
}

#[test]
fn empty_readme_gets_marker_only() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("README.md");
    fs::write(&path, "").unwrap();
    let content = ensure_marker_block(&path).unwrap();
    let expected = "<!-- bdg:begin -->\n<!-- bdg:end -->";
    assert_eq!(content, expected);
}

#[test]
fn multiple_markers_error_on_rewrite() {
    let input = "<!-- bdg:begin -->\n<!-- bdg:end -->\n<!-- bdg:begin -->\n<!-- bdg:end -->";
    let result = rewrite_marker_block(input, &[]);
    assert!(result.is_err());
}

#[test]
fn preserves_trailing_newline_and_crlf() {
    let input = "# T\r\n<!-- bdg:begin -->\r\n<!-- bdg:end -->\r\n";
    let updated = rewrite_marker_block(input, &["![a](a)".to_string()]).unwrap();
    assert!(updated.ends_with("\r\n"));
    assert!(updated.contains("\r\n![a](a)\r\n"));
}

#[test]
fn remove_all_keeps_markers() {
    let input = "<!-- bdg:begin -->\n![a](a)\n<!-- bdg:end -->";
    let updated = remove_marker_block(input).unwrap();
    let badges = extract_managed_block(&updated);
    assert!(badges.is_empty());
    assert!(!updated.contains("<!-- bdg:begin -->"));
    assert!(!updated.contains("<!-- bdg:end -->"));
}

#[test]
fn ignores_markers_inside_code_fence() {
    let input = "# Title\n```md\n<!-- bdg:begin -->\n```\n<!-- bdg:begin -->\n<!-- bdg:end -->";
    let updated = rewrite_marker_block(input, &["![a](a)".to_string()]).unwrap();
    assert!(updated.contains("![a](a)"));
}
