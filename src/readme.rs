use std::fs;
use std::path::{Path, PathBuf};

pub const BDG_BEGIN: &str = "<!-- bdg:begin -->";
pub const BDG_END: &str = "<!-- bdg:end -->";

pub fn resolve_readme(root: &Path, prefer_moonbit: bool) -> PathBuf {
    let candidates = if prefer_moonbit {
        vec!["README.mbt.md", "README.md", "docs/README.md"]
    } else {
        vec!["README.md", "README.mbt.md", "docs/README.md"]
    };
    for candidate in &candidates {
        let path = root.join(candidate);
        if path.exists() {
            return path;
        }
    }
    root.join(candidates[0])
}

pub fn ensure_marker_block(readme_path: &Path) -> anyhow::Result<String> {
    let content = fs::read_to_string(readme_path).unwrap_or_default();
    let (newline, has_trailing_newline) = detect_newline(&content);
    let lines = split_lines(&content, newline);
    let (begin_indices, end_indices) = collect_marker_indices(&lines);
    if begin_indices.len() == 1 && end_indices.len() == 1 {
        return Ok(content);
    }
    let mut lines = lines
        .into_iter()
        .map(|line| line.to_string())
        .collect::<Vec<_>>();
    let mut inserted = false;
    let mut in_code_fence = false;
    for idx in 0..lines.len() {
        if is_code_fence(&lines[idx]) {
            in_code_fence = !in_code_fence;
            continue;
        }
        if in_code_fence {
            continue;
        }
        if lines[idx].starts_with("# ") {
            let insert_at = idx + 1;
            lines.insert(insert_at, BDG_BEGIN.to_string());
            lines.insert(insert_at + 1, BDG_END.to_string());
            inserted = true;
            break;
        }
    }
    if !inserted {
        lines.insert(0, BDG_END.to_string());
        lines.insert(0, BDG_BEGIN.to_string());
    }
    Ok(join_lines(lines, newline, has_trailing_newline))
}

pub fn rewrite_marker_block(content: &str, badges: &[String]) -> anyhow::Result<String> {
    let (newline, has_trailing_newline) = detect_newline(content);
    let lines = split_lines(content, newline);
    let (begin_indices, end_indices) = collect_marker_indices(&lines);
    if begin_indices.len() != 1 || end_indices.len() != 1 {
        anyhow::bail!("marker block missing or duplicated");
    }
    let begin = begin_indices[0];
    let end = end_indices[0];
    if begin >= end {
        anyhow::bail!("invalid marker block");
    }

    let mut output: Vec<String> = Vec::new();
    output.extend(lines[..=begin].iter().map(|line| (*line).to_string()));
    for badge in badges {
        output.push(badge.to_string());
    }
    output.extend(lines[end..].iter().map(|line| (*line).to_string()));
    Ok(join_lines(output, newline, has_trailing_newline))
}

pub fn write_readme_atomic(readme_path: &Path, content: &str) -> anyhow::Result<()> {
    let tmp_path = readme_path.with_extension("bdg.tmp");
    fs::write(&tmp_path, content)?;
    fs::rename(tmp_path, readme_path)?;
    Ok(())
}

pub fn extract_managed_block(content: &str) -> Vec<String> {
    let (newline, _) = detect_newline(content);
    let lines = split_lines(content, newline);
    let (begin_indices, end_indices) = collect_marker_indices(&lines);
    if begin_indices.len() != 1 || end_indices.len() != 1 {
        return Vec::new();
    }
    let begin = begin_indices[0];
    let end = end_indices[0];
    if begin >= end {
        return Vec::new();
    }
    lines[begin + 1..end]
        .iter()
        .filter_map(|line| {
            if line.trim().is_empty() {
                None
            } else {
                Some((*line).to_string())
            }
        })
        .collect()
}

pub fn extract_marker_block_lines(content: &str) -> anyhow::Result<Vec<String>> {
    let (newline, _) = detect_newline(content);
    let lines = split_lines(content, newline);
    let (begin_indices, end_indices) = collect_marker_indices(&lines);
    if begin_indices.len() != 1 || end_indices.len() != 1 {
        anyhow::bail!("marker block missing or duplicated");
    }
    let begin = begin_indices[0];
    let end = end_indices[0];
    if begin >= end {
        anyhow::bail!("invalid marker block");
    }
    Ok(lines[begin + 1..end]
        .iter()
        .map(|s| (*s).to_string())
        .collect())
}

pub fn rewrite_marker_block_lines(content: &str, lines: &[String]) -> anyhow::Result<String> {
    let (newline, has_trailing_newline) = detect_newline(content);
    let content_lines = split_lines(content, newline);
    let (begin_indices, end_indices) = collect_marker_indices(&content_lines);
    if begin_indices.len() != 1 || end_indices.len() != 1 {
        anyhow::bail!("marker block missing or duplicated");
    }
    let begin = begin_indices[0];
    let end = end_indices[0];
    if begin >= end {
        anyhow::bail!("invalid marker block");
    }

    let mut output: Vec<String> = Vec::new();
    output.extend(
        content_lines[..=begin]
            .iter()
            .map(|line| (*line).to_string()),
    );
    output.extend(lines.iter().cloned());
    output.extend(content_lines[end..].iter().map(|line| (*line).to_string()));
    Ok(join_lines(output, newline, has_trailing_newline))
}

pub fn remove_marker_block(content: &str) -> anyhow::Result<String> {
    let (newline, has_trailing_newline) = detect_newline(content);
    let content_lines = split_lines(content, newline);
    let (begin_indices, end_indices) = collect_marker_indices(&content_lines);
    if begin_indices.len() != 1 || end_indices.len() != 1 {
        anyhow::bail!("marker block missing or duplicated");
    }
    let begin = begin_indices[0];
    let end = end_indices[0];
    if begin >= end {
        anyhow::bail!("invalid marker block");
    }
    let mut output: Vec<String> = Vec::new();
    output.extend(
        content_lines[..begin]
            .iter()
            .map(|line| (*line).to_string()),
    );
    output.extend(
        content_lines[end + 1..]
            .iter()
            .map(|line| (*line).to_string()),
    );
    Ok(join_lines(output, newline, has_trailing_newline))
}

fn detect_newline(content: &str) -> (&'static str, bool) {
    if content.contains("\r\n") {
        let trailing = content.ends_with("\r\n");
        return ("\r\n", trailing);
    }
    ("\n", content.ends_with('\n'))
}

fn split_lines<'a>(content: &'a str, newline: &str) -> Vec<&'a str> {
    if content.is_empty() {
        return Vec::new();
    }
    if newline == "\r\n" {
        content.split("\r\n").collect()
    } else {
        content.split('\n').collect()
    }
}

fn join_lines(lines: Vec<String>, newline: &str, trailing_newline: bool) -> String {
    let mut output = lines.join(newline);
    if trailing_newline {
        output.push_str(newline);
    }
    output
}

pub fn readme_newline_info(content: &str) -> (String, bool) {
    let (newline, trailing) = detect_newline(content);
    let label = if newline == "\r\n" { "CRLF" } else { "LF" };
    (label.to_string(), trailing)
}

pub fn marker_count(content: &str) -> usize {
    let (newline, _) = detect_newline(content);
    let lines = split_lines(content, newline);
    let (begin_indices, _) = collect_marker_indices(&lines);
    begin_indices.len()
}

fn collect_marker_indices(lines: &[&str]) -> (Vec<usize>, Vec<usize>) {
    let mut begin_indices = Vec::new();
    let mut end_indices = Vec::new();
    let mut in_code_fence = false;
    for (idx, line) in lines.iter().enumerate() {
        if is_code_fence(line) {
            in_code_fence = !in_code_fence;
            continue;
        }
        if in_code_fence {
            continue;
        }
        if *line == BDG_BEGIN {
            begin_indices.push(idx);
        }
        if *line == BDG_END {
            end_indices.push(idx);
        }
    }
    (begin_indices, end_indices)
}

fn is_code_fence(line: &str) -> bool {
    line.trim_start().starts_with("```")
}
