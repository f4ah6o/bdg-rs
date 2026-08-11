use crate::badges::{
    Badge, badge_for_codecov, badge_for_crates, badge_for_crates_downloads, badge_for_crates_msrv,
    badge_for_docs_rs, badge_for_docs_url, badge_for_github_downloads, badge_for_github_forks,
    badge_for_github_issues, badge_for_github_last_commit, badge_for_github_pull_requests,
    badge_for_github_release, badge_for_github_stars, badge_for_license, badge_for_license_text,
    badge_for_moonbit, badge_for_npm, badge_for_npm_downloads, badge_for_workflow, dedupe_badges,
};
use crate::config::{Config, load_config};
use crate::core::{ProjectContext, build_context};
use crate::inspect::build_list_json;
use crate::manifest::{read_moon_mod, read_resolved_cargo_package};
use crate::plan::ReadmePlan;
use crate::project::{infer_owner_repo, local_npm_packages, resolve_metadata};
use crate::readme::{
    ensure_marker_block, extract_managed_block, readme_newline_info, remove_marker_block,
    resolve_readme, rewrite_marker_block, rewrite_marker_block_lines,
};
use crate::readme_remove::remove_block_lines_by_id_kind;
use crate::version::VersionOptions;
use crate::workflows::{detect_workflows, detects_codecov, gh_latest_status_json_in};
use serde::Serialize;
use std::collections::HashSet;
use std::path::Path;

const BDG_SKILL: &str = include_str!("../.agents/skills/bdg/SKILL.md");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AddMode {
    Add,
    Sync,
}

pub fn cmd_add(
    current_dir: &Path,
    yes: bool,
    only: &[String],
    allow_yy_calver: bool,
    dry_run: bool,
    json: bool,
) -> anyhow::Result<i32> {
    cmd_add_inner(
        current_dir,
        AddMode::Add,
        yes,
        only,
        allow_yy_calver,
        dry_run,
        json,
    )
}

pub fn cmd_sync(
    current_dir: &Path,
    only: &[String],
    allow_yy_calver: bool,
    dry_run: bool,
    json: bool,
) -> anyhow::Result<i32> {
    cmd_add_inner(
        current_dir,
        AddMode::Sync,
        true,
        only,
        allow_yy_calver,
        dry_run,
        json,
    )
}

#[allow(clippy::too_many_arguments)]
fn cmd_add_inner(
    current_dir: &Path,
    mode: AddMode,
    yes: bool,
    only: &[String],
    allow_yy_calver: bool,
    dry_run: bool,
    json: bool,
) -> anyhow::Result<i32> {
    let context = build_context(current_dir)?;
    let config = load_config_for_context(current_dir, &context)?;
    let options = version_options(&context, Some((allow_yy_calver, &config)));
    let readme_path = resolve_readme(&context.root, context.has_moonbit());
    let npm_packages = local_npm_packages(&context);
    let metadata = resolve_metadata(&context, Some(&npm_packages))?;
    let (owner, repo) = infer_owner_repo(&metadata.repository);
    let workflows = detect_workflows(&context.root);

    let mut candidates = Vec::new();
    for package in npm_packages.iter().filter(|package| package.published) {
        candidates.push(badge_for_npm(&package.name));
        candidates.push(badge_for_npm_downloads(&package.name));
        if let Some(homepage) = package
            .registry
            .homepage
            .as_deref()
            .map(str::trim)
            .filter(|homepage| !homepage.is_empty())
        {
            candidates.push(badge_for_docs_url(homepage));
        }
    }
    if let Some(path) = &context.manifests.cargo_toml
        && let Ok(package) = read_resolved_cargo_package(path)
        && let Some(name) = package.and_then(|package| package.name)
    {
        candidates.push(badge_for_crates(&name));
        candidates.push(badge_for_crates_downloads(&name));
        candidates.push(badge_for_crates_msrv(&name));
        candidates.push(badge_for_docs_rs(&name));
    }
    if let Some(path) = &context.manifests.moon_mod
        && let Ok(module) = read_moon_mod(path)
        && let Some(name) = module.name.as_deref()
    {
        candidates.push(badge_for_moonbit(name));
        if name.contains('/') {
            candidates.push(badge_for_docs_url(&format!(
                "https://mooncakes.io/docs/{}",
                name
            )));
        }
    }
    if let Some(license) = metadata
        .license
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        candidates.push(badge_for_license_text(
            license,
            metadata.repository.as_deref(),
        ));
    } else if let (Some(owner), Some(repo)) = (owner.as_deref(), repo.as_deref()) {
        candidates.push(badge_for_license(owner, repo));
    }
    if let (Some(owner), Some(repo)) = (owner.as_deref(), repo.as_deref()) {
        candidates.push(badge_for_github_release(owner, repo));
        candidates.push(badge_for_github_downloads(owner, repo));
        candidates.push(badge_for_github_stars(owner, repo));
        candidates.push(badge_for_github_forks(owner, repo));
        candidates.push(badge_for_github_issues(owner, repo));
        candidates.push(badge_for_github_pull_requests(owner, repo));
        candidates.push(badge_for_github_last_commit(owner, repo));
        if detects_codecov(&context.root) {
            candidates.push(badge_for_codecov(owner, repo));
        }
        for workflow in workflows {
            candidates.push(badge_for_workflow(owner, repo, &workflow.file));
        }
    }

    let mut filtered = filter_badges(dedupe_badges(candidates), only, &config);
    if mode == AddMode::Sync && only.is_empty() {
        filtered.retain(|badge| badge.sync_default);
    }
    let selected = if yes {
        filtered
    } else if !only.is_empty() {
        match prompt_badges(&filtered)? {
            Some(selected) => selected,
            None => return Ok(0),
        }
    } else {
        let items: Vec<String> = filtered
            .iter()
            .map(|badge| format_badge_label(badge, &context, &options))
            .collect();
        let recommended = recommended_indices(&filtered);
        let selection = crate::tui::run_multi_select(
            "Select badges to add",
            Some("Recommended preselected: CI, version, license"),
            &items,
            &recommended,
        )?;
        if selection.cancelled {
            return Ok(0);
        }
        selection
            .selected
            .into_iter()
            .filter_map(|idx| filtered.get(idx).cloned())
            .collect()
    };
    let markdown: Vec<String> = selected.into_iter().map(|b| b.render_markdown()).collect();
    let content = ensure_marker_block(&readme_path)?;
    let updated = rewrite_marker_block(&content, &markdown)?;
    let plan = ReadmePlan::new(readme_path.clone(), content, updated);
    let diff = plan.diff();
    if dry_run {
        if json {
            let payload = DryRunJson {
                schema: "bdg.dryrun/v1".to_string(),
                path: plan.path().to_string_lossy().to_string(),
                diff: diff.clone(),
                removed_ids: None,
                missing_ids: None,
                removed_kinds: None,
                warnings: Vec::new(),
            };
            serde_json::to_writer_pretty(std::io::stdout(), &payload)?;
            println!();
        } else {
            print_diff(&diff);
        }
        return Ok(if diff.is_empty() { 0 } else { 2 });
    }
    plan.apply()?;
    Ok(0)
}

pub fn cmd_list(
    current_dir: &Path,
    json: bool,
    quiet: bool,
    allow_yy_calver: bool,
) -> anyhow::Result<()> {
    let context = build_context(current_dir)?;
    let config = load_config_for_context(current_dir, &context)?;
    let options = version_options(&context, Some((allow_yy_calver, &config)));
    let readme_path = resolve_readme(&context.root, context.has_moonbit());
    let content = if readme_path.exists() {
        std::fs::read_to_string(&readme_path)?
    } else {
        String::new()
    };
    let badges = extract_managed_block(&content);
    if json {
        let payload = build_list_json(
            &context,
            &readme_path,
            &content,
            &badges,
            &options,
            Some(&config),
        )?;
        serde_json::to_writer_pretty(std::io::stdout(), &payload)?;
        println!();
        return Ok(());
    }

    if !quiet {
        let (newline, trailing) = readme_newline_info(&content);
        let marker = crate::readme::marker_state(&content);
        let marker_label = if marker.is_valid() {
            "present"
        } else if marker.begin_count == 0 && marker.end_count == 0 {
            "missing"
        } else {
            "invalid"
        };
        println!(
            "README: {} ({}, trailing newline: {})",
            readme_path.to_string_lossy(),
            newline,
            if trailing { "yes" } else { "no" }
        );
        println!("Marker block: {marker_label}");
        println!("Badges: {}", badges.len());
        let workflows = detect_workflows(&context.root);
        for wf in workflows {
            let status = gh_latest_status_json_in(&context.root, &wf.file);
            if status.ok {
                if let Some(conclusion) = status.conclusion {
                    println!("- CI {} last: {}", wf.file, conclusion);
                }
            } else if let Some(reason) = status.reason {
                println!("- CI {} last: {}", wf.file, reason);
            }
        }
    }
    for badge in &badges {
        println!("{}", badge);
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn cmd_remove(
    current_dir: &Path,
    all: bool,
    ids: &[String],
    kinds: &[String],
    strict: bool,
    quiet: bool,
    dry_run: bool,
    json: bool,
    _allow_yy_calver: bool,
) -> anyhow::Result<i32> {
    let context = build_context(current_dir)?;
    let readme_path = resolve_readme(&context.root, context.has_moonbit());
    let content = ensure_marker_block(&readme_path)?;
    let existing = extract_managed_block(&content);
    if existing.is_empty() {
        return Ok(0);
    }
    if all && (!ids.is_empty() || !kinds.is_empty()) {
        anyhow::bail!("--all cannot be combined with --id or --kind");
    }

    let removal_result = if all {
        None
    } else if !ids.is_empty() || !kinds.is_empty() {
        Some(remove_block_lines_by_id_kind(&content, ids, kinds, strict)?)
    } else {
        None
    };
    let remaining = if all {
        Vec::new()
    } else if let Some(removal) = &removal_result {
        removal.remaining.clone()
    } else {
        let items = format_remove_items(&existing);
        let selection = crate::tui::run_multi_select("Select badges to remove", None, &items, &[])?;
        if selection.cancelled {
            return Ok(0);
        }
        let remove_set: HashSet<usize> = selection.selected.into_iter().collect();
        existing
            .into_iter()
            .enumerate()
            .filter_map(|(idx, badge)| {
                if remove_set.contains(&idx) {
                    None
                } else {
                    Some(badge)
                }
            })
            .collect()
    };
    let updated = if remaining.is_empty() {
        remove_marker_block(&content)?
    } else if removal_result.is_some() {
        rewrite_marker_block_lines(&content, &remaining)?
    } else {
        rewrite_marker_block(&content, &remaining)?
    };
    let plan = ReadmePlan::new(readme_path.clone(), content, updated);
    let diff = plan.diff();
    if let Some(removal) = &removal_result
        && !json
        && !quiet
    {
        print_remove_summary(
            readme_path.to_string_lossy().as_ref(),
            removal,
            remaining.len(),
        );
    }
    if dry_run {
        if json {
            let warnings = build_remove_warnings(removal_result.as_ref());
            let payload = DryRunJson {
                schema: "bdg.dryrun/v1".to_string(),
                path: plan.path().to_string_lossy().to_string(),
                diff: diff.clone(),
                removed_ids: removal_result.as_ref().map(|r| r.removed_ids.clone()),
                missing_ids: removal_result.as_ref().map(|r| r.missing_ids.clone()),
                removed_kinds: removal_result.as_ref().map(|r| r.removed_kinds.clone()),
                warnings,
            };
            serde_json::to_writer_pretty(std::io::stdout(), &payload)?;
            println!();
        } else {
            print_diff(&diff);
        }
        return Ok(if diff.is_empty() { 0 } else { 2 });
    }
    plan.apply()?;
    Ok(0)
}

pub fn cmd_skills() -> anyhow::Result<()> {
    print!("{}", BDG_SKILL);
    Ok(())
}

fn prompt_badges(badges: &[Badge]) -> anyhow::Result<Option<Vec<Badge>>> {
    if badges.is_empty() {
        return Ok(Some(Vec::new()));
    }
    let items: Vec<String> = badges.iter().map(|b| b.render_markdown()).collect();
    let selection = crate::tui::run_multi_select("Select badges to add", None, &items, &[])?;
    if selection.cancelled {
        return Ok(None);
    }
    let chosen = selection
        .selected
        .into_iter()
        .filter_map(|idx| badges.get(idx).cloned())
        .collect();
    Ok(Some(chosen))
}

fn filter_badges(badges: Vec<Badge>, only: &[String], config: &Config) -> Vec<Badge> {
    if only.is_empty() {
        let excluded: HashSet<String> = config
            .badges
            .exclude
            .iter()
            .map(|s| s.trim().to_lowercase())
            .collect();
        return badges
            .into_iter()
            .filter(|badge| !excluded.contains(badge.kind.as_str()))
            .collect();
    }
    let only_lower: HashSet<String> = only.iter().map(|s| s.trim().to_lowercase()).collect();
    badges
        .into_iter()
        .filter(|badge| only_lower.contains(badge.kind.as_str()))
        .collect()
}

fn format_badge_label(badge: &Badge, context: &ProjectContext, options: &VersionOptions) -> String {
    match badge.kind {
        crate::badges::BadgeKind::Ci => {
            let workflow = badge
                .image_url
                .split("/actions/workflows/")
                .nth(1)
                .and_then(|rest| rest.split('/').next())
                .unwrap_or("workflow");
            let status = gh_latest_status_json_in(&context.root, workflow);
            if status.ok
                && let Some(conclusion) = status.conclusion
            {
                return format!("CI ({}) last: {}", workflow, conclusion);
            }
            format!("CI ({})", workflow)
        }
        crate::badges::BadgeKind::Version => {
            let version = extract_version_from_badge(badge);
            if let Some(version) = version {
                let info = crate::version::classify_version(&version, options);
                format!(
                    "{} version ({}, {})",
                    badge.label, version, info.version_format
                )
            } else {
                format!("{} version", badge.label)
            }
        }
        crate::badges::BadgeKind::License => "license".to_string(),
        crate::badges::BadgeKind::Release => "release".to_string(),
        crate::badges::BadgeKind::Docs => "docs".to_string(),
        crate::badges::BadgeKind::Downloads => badge.label.clone(),
        crate::badges::BadgeKind::Coverage => "coverage".to_string(),
        crate::badges::BadgeKind::Msrv => "MSRV".to_string(),
        crate::badges::BadgeKind::Stars => "GitHub stars".to_string(),
        crate::badges::BadgeKind::Forks => "GitHub forks".to_string(),
        crate::badges::BadgeKind::Issues => "GitHub issues".to_string(),
        crate::badges::BadgeKind::PullRequests => "GitHub pull requests".to_string(),
        crate::badges::BadgeKind::Activity => "GitHub last commit".to_string(),
    }
}

fn extract_version_from_badge(badge: &Badge) -> Option<String> {
    if let Some(url) = badge.image_url.split("img.shields.io/npm/v/").nth(1) {
        let segment = url.split(&['/', '?'][..]).next().unwrap_or("");
        return Some(segment.trim_end_matches(".svg").to_string());
    }
    if let Some(url) = badge.image_url.split("img.shields.io/crates/v/").nth(1) {
        let segment = url.split(&['/', '?'][..]).next().unwrap_or("");
        return Some(segment.trim_end_matches(".svg").to_string());
    }
    None
}

fn load_config_for_context(current_dir: &Path, context: &ProjectContext) -> anyhow::Result<Config> {
    load_config(current_dir, &context.root)
}

fn version_options(
    _context: &ProjectContext,
    override_allow_yy: Option<(bool, &Config)>,
) -> VersionOptions {
    let allow_yy = override_allow_yy.map(|(flag, _)| flag).unwrap_or(false);
    let config = override_allow_yy.map(|(_, cfg)| cfg);
    let version_cfg = config.map(|cfg| &cfg.version);
    VersionOptions {
        allow_yy_calver: if override_allow_yy.is_some() {
            allow_yy
        } else {
            version_cfg.map(|v| v.allow_yy_calver).unwrap_or(false)
        },
        year_min: version_cfg.map(|v| v.year_min).unwrap_or(2000),
        year_max: version_cfg.map(|v| v.year_max).unwrap_or(2199),
    }
}

fn recommended_indices(badges: &[Badge]) -> Vec<usize> {
    let mut selected = Vec::new();
    if let Some((idx, _)) = badges
        .iter()
        .enumerate()
        .find(|(_, badge)| badge.kind == crate::badges::BadgeKind::Ci)
    {
        selected.push(idx);
    }
    for (idx, badge) in badges.iter().enumerate() {
        if badge.kind == crate::badges::BadgeKind::Version
            && (badge.label.contains("crates")
                || badge.label.contains("npm")
                || badge.label.contains("moonbit"))
        {
            selected.push(idx);
        }
    }
    if let Some((idx, _)) = badges
        .iter()
        .enumerate()
        .find(|(_, badge)| badge.kind == crate::badges::BadgeKind::License)
    {
        selected.push(idx);
    }
    selected.sort();
    selected.dedup();
    selected
}

fn format_remove_items(lines: &[String]) -> Vec<String> {
    lines
        .iter()
        .map(|line| {
            if let Some(parsed) = crate::readme_badges::parse_badge_line_optional(line) {
                let mut summary = format!("{} [{}]", parsed.kind, parsed.id);
                if !parsed.label.is_empty() {
                    summary.push_str(&format!(" \"{}\"", parsed.label));
                }
                if !parsed.image.is_empty() {
                    let image = shorten(&parsed.image, 48);
                    summary.push_str(&format!(" {}", image));
                }
                summary
            } else {
                format!("unknown {}", shorten(line, 48))
            }
        })
        .collect()
}

fn shorten(text: &str, max: usize) -> String {
    if text.len() <= max {
        return text.to_string();
    }
    let mut output = text.chars().take(max).collect::<String>();
    output.push('…');
    output
}

#[derive(Debug, Serialize)]
struct WarningJson {
    code: String,
    message: String,
    meta: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct DryRunJson {
    schema: String,
    path: String,
    diff: String,
    removed_ids: Option<Vec<String>>,
    missing_ids: Option<Vec<String>>,
    removed_kinds: Option<std::collections::HashMap<String, usize>>,
    warnings: Vec<WarningJson>,
}

fn print_diff(diff: &str) {
    if diff.is_empty() {
        return;
    }
    print!("{}", diff);
}

fn print_remove_summary(
    path: &str,
    removal: &crate::readme_remove::RemovalOutcome,
    remaining: usize,
) {
    println!("Removed {} badges from {}", removal.removed, path);
    if !removal.removed_ids.is_empty() {
        let ids_summary = summarize_items(&removal.removed_ids, 20);
        println!("- ids: {}", ids_summary);
    }
    if !removal.removed_kinds.is_empty() {
        let mut pairs = removal
            .removed_kinds
            .iter()
            .map(|(kind, count)| format!("{}={}", kind, count))
            .collect::<Vec<_>>();
        pairs.sort();
        println!("- kinds: {}", pairs.join(", "));
    }
    println!("Remaining: {}", remaining);
}

fn summarize_items(items: &[String], max: usize) -> String {
    if items.len() <= max {
        return items.join(", ");
    }
    let shown = items[..max].join(", ");
    format!("{} …+{}", shown, items.len() - max)
}

fn build_remove_warnings(
    removal: Option<&crate::readme_remove::RemovalOutcome>,
) -> Vec<WarningJson> {
    let mut warnings = Vec::new();
    if let Some(removal) = removal {
        for missing in &removal.missing_ids {
            warnings.push(WarningJson {
                code: "ID_NOT_FOUND".to_string(),
                message: "badge id not found in readme_block".to_string(),
                meta: Some(serde_json::json!({ "id": missing })),
            });
        }
    }
    warnings
}
