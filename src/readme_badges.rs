use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct ParsedBadge {
    pub id: String,
    pub kind: String,
    pub label: String,
    pub image: String,
    pub link: Option<String>,
    pub source: String,
    pub meta: Option<serde_json::Value>,
    pub raw: String,
}

pub fn parse_badge_line(line: &str) -> ParsedBadge {
    if let Some((label, image, link)) = parse_linked_image(line) {
        return build_badge(line, label, image, Some(link));
    }
    if let Some((label, image)) = parse_image(line) {
        return build_badge(line, label, image, None);
    }
    ParsedBadge {
        id: format!("unknown:{}", hash_line(line)),
        kind: "unknown".to_string(),
        label: String::new(),
        image: String::new(),
        link: None,
        source: "readme".to_string(),
        meta: None,
        raw: line.to_string(),
    }
}

pub fn parse_badge_line_optional(line: &str) -> Option<ParsedBadge> {
    if let Some((label, image, link)) = parse_linked_image(line) {
        return Some(build_badge(line, label, image, Some(link)));
    }
    if let Some((label, image)) = parse_image(line) {
        return Some(build_badge(line, label, image, None));
    }
    None
}

fn build_badge(raw: &str, label: String, image: String, link: Option<String>) -> ParsedBadge {
    let (kind, id, meta) = infer_kind(&image, raw);
    ParsedBadge {
        id,
        kind,
        label,
        image,
        link,
        source: "readme".to_string(),
        meta,
        raw: raw.to_string(),
    }
}

fn parse_linked_image(line: &str) -> Option<(String, String, String)> {
    let trimmed = line.trim();
    if !trimmed.starts_with("[![") || !trimmed.ends_with(')') {
        return None;
    }
    let start_label = trimmed.find("[![")? + 3;
    let end_label = trimmed[start_label..].find("](")? + start_label;
    let label = trimmed[start_label..end_label].to_string();

    let start_image = end_label + 2;
    let end_image = trimmed[start_image..].find(")]")? + start_image;
    let image = trimmed[start_image..end_image].to_string();

    let start_link = trimmed[end_image + 2..].find("(")? + end_image + 3;
    let end_link = trimmed[start_link..].find(")")? + start_link;
    let link = trimmed[start_link..end_link].to_string();
    if image.is_empty() {
        return None;
    }
    Some((label, image.trim().to_string(), link.trim().to_string()))
}

fn parse_image(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    if !trimmed.starts_with("![") || !trimmed.ends_with(')') {
        return None;
    }
    let start_label = 2;
    let end_label = trimmed[start_label..].find("](")? + start_label;
    let label = trimmed[start_label..end_label].to_string();
    let start_image = end_label + 2;
    let end_image = trimmed[start_image..].find(")")? + start_image;
    let image = trimmed[start_image..end_image].to_string();
    if image.is_empty() {
        return None;
    }
    Some((label, image.trim().to_string()))
}

fn infer_kind(image: &str, raw: &str) -> (String, String, Option<serde_json::Value>) {
    let image_trimmed = image.trim();
    if !is_http_url(image_trimmed) {
        return (
            "unknown".to_string(),
            format!("unknown:{}", hash_line(raw)),
            None,
        );
    }
    if image_trimmed.contains("/actions/workflows/") && image_trimmed.contains("/badge.svg") {
        let workflow_file = image_trimmed
            .split("/actions/workflows/")
            .nth(1)
            .and_then(|rest| rest.split('/').next())
            .map(|s| s.to_string());
        if let Some(file) = workflow_file {
            return (
                "github_actions".to_string(),
                format!("ci:{}", file),
                Some(serde_json::json!({ "workflow_file": file })),
            );
        }
        return (
            "github_actions".to_string(),
            format!("unknown:{}", hash_line(raw)),
            None,
        );
    }
    if let Some(pkg) = extract_after_prefix(image_trimmed, "img.shields.io/npm/v/") {
        return (
            "npm_version".to_string(),
            format!("npm:{}", pkg),
            Some(serde_json::json!({ "package": pkg })),
        );
    }
    if let Some(pkg) = extract_after_prefix(image_trimmed, "img.shields.io/npm/dw/")
        .or_else(|| extract_after_prefix(image_trimmed, "img.shields.io/npm/dm/"))
        .or_else(|| extract_after_prefix(image_trimmed, "img.shields.io/npm/dt/"))
    {
        return (
            "npm_downloads".to_string(),
            format!("npm_downloads:{}", pkg),
            Some(serde_json::json!({ "package": pkg })),
        );
    }
    if let Some(crate_name) = extract_after_prefix(image_trimmed, "img.shields.io/crates/v/") {
        return (
            "crates_version".to_string(),
            format!("crates:{}", crate_name),
            Some(serde_json::json!({ "crate": crate_name })),
        );
    }
    if let Some(crate_name) = extract_after_prefix(image_trimmed, "img.shields.io/crates/d/") {
        return (
            "crates_downloads".to_string(),
            format!("crates_downloads:{}", crate_name),
            Some(serde_json::json!({ "crate": crate_name })),
        );
    }
    if image_trimmed.contains("img.shields.io/github/license/") {
        return ("license".to_string(), "license:github".to_string(), None);
    }
    if image_trimmed.contains("img.shields.io/github/v/release/") {
        return (
            "github_release".to_string(),
            "release:github".to_string(),
            None,
        );
    }
    if let Some((owner, repo)) = extract_codecov_repo(image_trimmed) {
        return (
            "coverage".to_string(),
            "coverage:codecov".to_string(),
            Some(serde_json::json!({ "owner": owner, "repo": repo })),
        );
    }
    if let Some((label, message)) = extract_custom_badge(image_trimmed) {
        if label.eq_ignore_ascii_case("docs") {
            return (
                "docs".to_string(),
                "docs:custom".to_string(),
                Some(serde_json::json!({ "label": label, "message": message })),
            );
        }
    }
    (
        "unknown".to_string(),
        format!("unknown:{}", hash_line(raw)),
        None,
    )
}

fn extract_after_prefix(image: &str, prefix: &str) -> Option<String> {
    let pos = image.find(prefix)?;
    let remainder = &image[pos + prefix.len()..];
    let before_query = remainder.split('?').next().unwrap_or("");
    let trimmed = before_query.trim_end_matches(".svg");
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn extract_codecov_repo(image: &str) -> Option<(String, String)> {
    let prefix = "img.shields.io/codecov/c/github/";
    let pos = image.find(prefix)?;
    let remainder = &image[pos + prefix.len()..];
    let before_query = remainder.split('?').next().unwrap_or("");
    let mut parts = before_query.split('/');
    let owner = parts.next()?.to_string();
    let repo = parts.next()?.trim_end_matches(".svg").to_string();
    if owner.is_empty() || repo.is_empty() {
        None
    } else {
        Some((owner, repo))
    }
}

fn extract_custom_badge(image: &str) -> Option<(String, String)> {
    let prefix = "img.shields.io/badge/";
    let pos = image.find(prefix)?;
    let remainder = &image[pos + prefix.len()..];
    let before_query = remainder.split('?').next().unwrap_or("");
    let mut parts = before_query.split('-');
    let label = parts.next()?.to_string();
    let message = parts.next().unwrap_or("").to_string();
    if label.is_empty() {
        None
    } else {
        Some((label, message))
    }
}

fn is_http_url(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}

fn hash_line(line: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    line.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}
