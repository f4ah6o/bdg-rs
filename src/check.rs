use crate::core::build_context;
use crate::readme::{extract_managed_block, marker_state, resolve_readme};
use crate::readme_badges::parse_badge_line_optional;
use serde::Serialize;
use std::collections::HashSet;
use std::path::Path;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CheckIssue {
    pub level: &'static str,
    pub code: &'static str,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CheckReport {
    pub schema: &'static str,
    pub path: String,
    pub ok: bool,
    pub marker: MarkerReport,
    pub badge_count: usize,
    pub issues: Vec<CheckIssue>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MarkerReport {
    pub begin_count: usize,
    pub end_count: usize,
    pub ordered: bool,
}

pub fn inspect_readme(path: &Path, content: &str, strict: bool) -> CheckReport {
    let marker = marker_state(content);
    let mut issues = Vec::new();

    if marker.begin_count == 0 && marker.end_count == 0 {
        issues.push(CheckIssue {
            level: "error",
            code: "MARKER_MISSING",
            message: "managed badge block is missing".to_string(),
        });
    } else if !marker.is_valid() {
        issues.push(CheckIssue {
            level: "error",
            code: "MARKER_INVALID",
            message: format!(
                "expected one ordered marker pair, found begin={} end={} ordered={}",
                marker.begin_count, marker.end_count, marker.ordered
            ),
        });
    }

    let badges = if marker.is_valid() {
        extract_managed_block(content)
    } else {
        Vec::new()
    };
    let mut ids = HashSet::new();
    for line in &badges {
        match parse_badge_line_optional(line) {
            Some(parsed) if parsed.kind != "unknown" => {
                if !ids.insert(parsed.id.clone()) {
                    issues.push(CheckIssue {
                        level: "error",
                        code: "DUPLICATE_BADGE",
                        message: format!("duplicate managed badge id `{}`", parsed.id),
                    });
                }
            }
            _ => issues.push(CheckIssue {
                level: if strict { "error" } else { "warning" },
                code: "UNKNOWN_BADGE",
                message: format!("unrecognized managed line: {line}"),
            }),
        }
    }

    let ok = !issues.iter().any(|issue| issue.level == "error");
    CheckReport {
        schema: "bdg.check/v1",
        path: path.to_string_lossy().to_string(),
        ok,
        marker: MarkerReport {
            begin_count: marker.begin_count,
            end_count: marker.end_count,
            ordered: marker.ordered,
        },
        badge_count: badges.len(),
        issues,
    }
}

pub fn cmd_check(current_dir: &Path, json: bool, strict: bool) -> anyhow::Result<i32> {
    let context = build_context(current_dir)?;
    let readme_path = resolve_readme(&context.root, context.has_moonbit());
    let content = if readme_path.exists() {
        std::fs::read_to_string(&readme_path)?
    } else {
        String::new()
    };
    let mut report = inspect_readme(&readme_path, &content, strict);
    if !readme_path.exists() {
        report.issues.insert(
            0,
            CheckIssue {
                level: "error",
                code: "README_MISSING",
                message: "README file does not exist".to_string(),
            },
        );
        report.ok = false;
    }

    if json {
        serde_json::to_writer_pretty(std::io::stdout(), &report)?;
        println!();
    } else {
        println!(
            "{}: {} ({} badges)",
            report.path,
            if report.ok { "ok" } else { "invalid" },
            report.badge_count
        );
        for issue in &report.issues {
            println!("{} [{}] {}", issue.level, issue.code, issue.message);
        }
    }

    Ok(if report.ok { 0 } else { 1 })
}

#[cfg(test)]
mod tests {
    use super::inspect_readme;
    use std::path::Path;

    #[test]
    fn accepts_valid_managed_badges() {
        let report = inspect_readme(
            Path::new("README.md"),
            "# demo\n<!-- bdg:begin -->\n[![CI](https://github.com/o/r/actions/workflows/ci.yml/badge.svg)](https://github.com/o/r/actions/workflows/ci.yml)\n<!-- bdg:end -->\n",
            true,
        );
        assert!(report.ok);
        assert_eq!(report.badge_count, 1);
        assert!(report.issues.is_empty());
    }

    #[test]
    fn strict_mode_rejects_unknown_lines() {
        let report = inspect_readme(
            Path::new("README.md"),
            "<!-- bdg:begin -->\nplain text\n<!-- bdg:end -->\n",
            true,
        );
        assert!(!report.ok);
        assert_eq!(report.issues[0].code, "UNKNOWN_BADGE");
    }

    #[test]
    fn detects_missing_and_duplicate_markers() {
        let missing = inspect_readme(Path::new("README.md"), "# demo\n", false);
        assert!(!missing.ok);
        assert_eq!(missing.issues[0].code, "MARKER_MISSING");

        let duplicate = inspect_readme(
            Path::new("README.md"),
            "<!-- bdg:begin -->\n<!-- bdg:begin -->\n<!-- bdg:end -->\n",
            false,
        );
        assert!(!duplicate.ok);
        assert_eq!(duplicate.issues[0].code, "MARKER_INVALID");
    }
}
