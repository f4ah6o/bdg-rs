use crate::badges::{
    badge_for_crates, badge_for_license, badge_for_moonbit, badge_for_npm, badge_for_workflow,
    Badge,
};
use crate::config::{load_config, Config};
use crate::core::{build_context, Ecosystem, ProjectContext};
use crate::manifest::{read_cargo_toml, read_moon_mod, read_package_json, RepositoryField};
use crate::providers::{fetch_crates_metadata, fetch_npm_metadata, RegistryMetadata};
use crate::readme::{
    ensure_marker_block, extract_managed_block, readme_newline_info, remove_marker_block,
    resolve_readme, rewrite_marker_block, rewrite_marker_block_lines, write_readme_atomic,
    BDG_BEGIN, BDG_END,
};
use crate::readme_badges::ParsedBadge;
use crate::readme_remove::remove_block_lines_by_id_kind;
use crate::version::VersionOptions;
use crate::workflows::{detect_workflows, gh_latest_status_json, WorkflowInfo};
use anyhow::Context;
use dialoguer::MultiSelect;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct ResolvedMetadata {
    pub name: Option<String>,
    pub version: Option<String>,
    pub license: Option<String>,
    pub repository: Option<String>,
    pub description: Option<String>,
    pub registry: Option<RegistryMetadata>,
}

pub fn cmd_add(
    current_dir: &Path,
    yes: bool,
    only: &[String],
    allow_yy_calver: bool,
    dry_run: bool,
    json: bool,
) -> anyhow::Result<i32> {
    let context = build_context(current_dir)?;
    let config = load_config_for_context(&context)?;
    let options = version_options(&context, Some((allow_yy_calver, &config)));
    let readme_path = resolve_readme(&context.root, context.has_moonbit());
    let metadata = resolve_metadata(&context)?;
    let (owner, repo) = infer_owner_repo(&metadata.repository);
    let workflows = detect_workflows(&context.root);

    let mut candidates = Vec::new();
    if let Some(path) = &context.manifests.package_json {
        if let Ok(pkg) = read_package_json(path) {
            if let Some(name) = pkg.name.as_deref() {
                candidates.push(badge_for_npm(name));
            }
        }
    }
    if let Some(path) = &context.manifests.cargo_toml {
        if let Ok(manifest) = read_cargo_toml(path) {
            if let Some(package) = manifest.package {
                if let Some(name) = package.name.as_deref() {
                    candidates.push(badge_for_crates(name));
                }
            }
        }
    }
    if let Some(path) = &context.manifests.moon_mod {
        if let Ok(module) = read_moon_mod(path) {
            if let Some(name) = module.name.as_deref() {
                candidates.push(badge_for_moonbit(name));
            }
        }
    }
    if let (Some(owner), Some(repo)) = (owner.as_deref(), repo.as_deref()) {
        candidates.push(badge_for_license(owner, repo));
        for workflow in workflows {
            candidates.push(badge_for_workflow(owner, repo, &workflow.name));
        }
    }

    let filtered = filter_badges(candidates, only);
    let selected = if yes {
        filtered
    } else if !only.is_empty() {
        prompt_badges(&filtered)?
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
    let diff = unified_diff(&readme_path, &content, &updated);
    if dry_run {
        if json {
            let payload = DryRunJson {
                schema: "bdg.dryrun/v1".to_string(),
                path: readme_path.to_string_lossy().to_string(),
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
    write_readme_atomic(&readme_path, &updated)?;
    Ok(0)
}

pub fn cmd_list(
    current_dir: &Path,
    json: bool,
    quiet: bool,
    allow_yy_calver: bool,
) -> anyhow::Result<()> {
    let context = build_context(current_dir)?;
    let config = load_config_for_context(&context)?;
    let options = version_options(&context, Some((allow_yy_calver, &config)));
    let readme_path = resolve_readme(&context.root, context.has_moonbit());
    let content = ensure_marker_block(&readme_path)?;
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
        let marker_present = content.contains(BDG_BEGIN) && content.contains(BDG_END);
        println!(
            "README: {} ({}, trailing newline: {})",
            readme_path.to_string_lossy(),
            newline,
            if trailing { "yes" } else { "no" }
        );
        println!(
            "Marker block: {}",
            if marker_present { "present" } else { "missing" }
        );
        println!("Badges: {}", badges.len());
        let workflows = detect_workflows(&context.root);
        for wf in workflows {
            let status = gh_latest_status_json(&wf.file);
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
    let diff = unified_diff(&readme_path, &content, &updated);
    if let Some(removal) = &removal_result {
        if !json && !quiet {
            print_remove_summary(
                readme_path.to_string_lossy().as_ref(),
                removal,
                remaining.len(),
            );
        }
    }
    if dry_run {
        if json {
            let warnings = build_remove_warnings(removal_result.as_ref());
            let payload = DryRunJson {
                schema: "bdg.dryrun/v1".to_string(),
                path: readme_path.to_string_lossy().to_string(),
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
    write_readme_atomic(&readme_path, &updated)?;
    Ok(0)
}

fn prompt_badges(badges: &[Badge]) -> anyhow::Result<Vec<Badge>> {
    if badges.is_empty() {
        return Ok(Vec::new());
    }
    let items: Vec<String> = badges.iter().map(|b| b.render_markdown()).collect();
    let selections = MultiSelect::new()
        .with_prompt("Select badges to add")
        .items(&items)
        .interact()?;
    let chosen = selections
        .into_iter()
        .filter_map(|idx| badges.get(idx).cloned())
        .collect();
    Ok(chosen)
}

fn filter_badges(badges: Vec<Badge>, only: &[String]) -> Vec<Badge> {
    if only.is_empty() {
        return badges;
    }
    let only_lower: HashSet<String> = only.iter().map(|s| s.trim().to_lowercase()).collect();
    badges
        .into_iter()
        .filter(|badge| match badge.kind {
            crate::badges::BadgeKind::Ci => only_lower.contains("ci"),
            crate::badges::BadgeKind::Version => only_lower.contains("version"),
            crate::badges::BadgeKind::License => only_lower.contains("license"),
            crate::badges::BadgeKind::Release => only_lower.contains("release"),
            crate::badges::BadgeKind::Docs => only_lower.contains("docs"),
            crate::badges::BadgeKind::Downloads => only_lower.contains("downloads"),
        })
        .collect()
}

fn format_badge_label(
    badge: &Badge,
    _context: &ProjectContext,
    options: &VersionOptions,
) -> String {
    match badge.kind {
        crate::badges::BadgeKind::Ci => {
            let workflow = badge
                .image_url
                .split("/actions/workflows/")
                .nth(1)
                .and_then(|rest| rest.split('/').next())
                .unwrap_or("workflow");
            let status = crate::workflows::gh_latest_status_json(workflow);
            if status.ok {
                if let Some(conclusion) = status.conclusion {
                    return format!("CI ({}) last: {}", workflow, conclusion);
                }
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
        crate::badges::BadgeKind::Downloads => "downloads".to_string(),
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

fn load_config_for_context(context: &ProjectContext) -> anyhow::Result<Config> {
    load_config(&std::env::current_dir()?, &context.root)
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

fn resolve_metadata(context: &ProjectContext) -> anyhow::Result<ResolvedMetadata> {
    match context.ecosystem {
        Some(Ecosystem::Node) => resolve_node_metadata(context),
        Some(Ecosystem::MoonBit) => resolve_moonbit_metadata(context),
        Some(Ecosystem::Rust) => resolve_rust_metadata(context),
        None => Ok(ResolvedMetadata::default()),
    }
}

fn resolve_node_metadata(context: &ProjectContext) -> anyhow::Result<ResolvedMetadata> {
    let manifest_path = context
        .manifests
        .package_json
        .as_ref()
        .context("package.json missing")?;
    let package = read_package_json(manifest_path)?;
    let registry = package
        .name
        .as_deref()
        .and_then(|name| fetch_npm_metadata(name).ok())
        .unwrap_or_else(RegistryMetadata::empty);
    Ok(ResolvedMetadata {
        name: package.name,
        version: registry.version.clone().or(package.version),
        license: registry.license.clone().or(package.license),
        repository: registry
            .repository
            .clone()
            .or_else(|| repository_to_string(package.repository)),
        description: registry.description.clone().or(package.description),
        registry: Some(registry),
    })
}

fn resolve_moonbit_metadata(context: &ProjectContext) -> anyhow::Result<ResolvedMetadata> {
    let manifest_path = context
        .manifests
        .moon_mod
        .as_ref()
        .context("moon.mod.json missing")?;
    let module = read_moon_mod(manifest_path)?;
    Ok(ResolvedMetadata {
        name: module.name,
        version: module.version,
        license: None,
        repository: None,
        description: None,
        registry: None,
    })
}

fn resolve_rust_metadata(context: &ProjectContext) -> anyhow::Result<ResolvedMetadata> {
    let manifest_path = context
        .manifests
        .cargo_toml
        .as_ref()
        .context("Cargo.toml missing")?;
    let manifest = read_cargo_toml(manifest_path)?;
    let package = manifest.package.unwrap_or(crate::manifest::CargoPackage {
        name: None,
        version: None,
        description: None,
        license: None,
        repository: None,
    });
    let registry = package
        .name
        .as_deref()
        .and_then(|name| fetch_crates_metadata(name).ok())
        .unwrap_or_else(RegistryMetadata::empty);
    Ok(ResolvedMetadata {
        name: package.name,
        version: registry.version.clone().or(package.version),
        license: registry.license.clone().or(package.license),
        repository: registry.repository.clone().or(package.repository),
        description: registry.description.clone().or(package.description),
        registry: Some(registry),
    })
}

fn infer_owner_repo(repository: &Option<String>) -> (Option<String>, Option<String>) {
    let url = match repository {
        Some(url) => url,
        None => return (None, None),
    };
    let cleaned = url
        .trim()
        .trim_end_matches(".git")
        .replace("git+", "")
        .replace("git://", "https://");
    let parts: Vec<&str> = cleaned.split('/').collect();
    if parts.len() < 2 {
        return (None, None);
    }
    let repo = parts.last().unwrap_or(&"").to_string();
    let owner = parts.get(parts.len() - 2).unwrap_or(&"").to_string();
    if owner.is_empty() || repo.is_empty() {
        return (None, None);
    }
    (Some(owner), Some(repo))
}

fn repository_to_string(repo: Option<RepositoryField>) -> Option<String> {
    match repo {
        Some(RepositoryField::String(value)) => Some(value),
        Some(RepositoryField::Object { url }) => url,
        None => None,
    }
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

fn unified_diff(path: &std::path::Path, original: &str, updated: &str) -> String {
    if original == updated {
        return String::new();
    }
    let rel_path = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("README.md");
    let patch = diffy::create_patch(original, updated);
    let formatted = diffy::PatchFormatter::new().fmt_patch(&patch).to_string();
    formatted
        .replace("--- original\n", &format!("--- a/{}\n", rel_path))
        .replace("+++ modified\n", &format!("+++ b/{}\n", rel_path))
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

#[derive(Debug, Serialize)]
struct ListJson {
    schema: String,
    repo: Option<RepoJson>,
    config: Option<ConfigJson>,
    readme: ReadmeJson,
    manifests: HashMap<String, serde_json::Value>,
    registries: HashMap<String, serde_json::Value>,
    ci: CiJson,
    readme_block: ReadmeBlockJson,
    warnings: Vec<WarningJson>,
}

#[derive(Debug, Serialize)]
struct ConfigJson {
    version: ConfigVersionJson,
}

#[derive(Debug, Serialize)]
struct ConfigVersionJson {
    allow_yy_calver: bool,
    year_min: i32,
    year_max: i32,
}

#[derive(Debug, Serialize)]
struct RepoJson {
    git_root: String,
    remote: Option<String>,
    owner: Option<String>,
    name: Option<String>,
    default_branch: Option<String>,
}

#[derive(Debug, Serialize)]
struct ReadmeJson {
    path: String,
    newline: String,
    trailing_newline: bool,
    markers: MarkerJson,
}

#[derive(Debug, Serialize)]
struct MarkerJson {
    present: bool,
    count: usize,
}

#[derive(Debug, Serialize)]
struct CiJson {
    workflows_dir: String,
    workflows: Vec<WorkflowJson>,
}

#[derive(Debug, Serialize)]
struct WorkflowJson {
    file: String,
    name: String,
    badge: WorkflowBadgeJson,
    latest_status: GhStatusJson,
}

#[derive(Debug, Serialize)]
struct WorkflowBadgeJson {
    kind: String,
    image: String,
    link: String,
}

#[derive(Debug, Serialize)]
struct GhStatusJson {
    source: String,
    ok: bool,
    reason: Option<String>,
    conclusion: Option<String>,
    run_id: Option<u64>,
    html_url: Option<String>,
    updated_at: Option<String>,
}

#[derive(Debug, Serialize)]
struct ReadmeBlockJson {
    raw: String,
    badges: Vec<ReadmeBadgeJson>,
}

#[derive(Debug, Serialize)]
struct ReadmeBadgeJson {
    id: String,
    kind: String,
    label: String,
    image: String,
    link: Option<String>,
    source: String,
    meta: Option<serde_json::Value>,
    raw: String,
}

#[derive(Debug, Serialize)]
struct WarningJson {
    code: String,
    message: String,
    meta: Option<serde_json::Value>,
}

fn build_list_json(
    context: &ProjectContext,
    readme_path: &std::path::Path,
    content: &str,
    badges: &[String],
    options: &VersionOptions,
    config: Option<&Config>,
) -> anyhow::Result<ListJson> {
    let repo = context.git.as_ref().map(|git| RepoJson {
        git_root: git.root.to_string_lossy().to_string(),
        remote: git.remote.clone(),
        owner: git.owner.clone(),
        name: git.repo.clone(),
        default_branch: git.default_branch.clone(),
    });

    let (newline, trailing) = readme_newline_info(content);
    let marker_count = crate::readme::marker_count(content);
    let readme_json = ReadmeJson {
        path: readme_path.to_string_lossy().to_string(),
        newline,
        trailing_newline: trailing,
        markers: MarkerJson {
            present: content.contains(BDG_BEGIN) && content.contains(BDG_END),
            count: marker_count,
        },
    };

    let manifests = collect_manifests(context, options)?;
    let registries = collect_registries(context, options)?;
    let ci = build_ci_json(context)?;
    let readme_block = build_readme_block(badges);

    let config_json = config.map(|cfg| ConfigJson {
        version: ConfigVersionJson {
            allow_yy_calver: cfg.version.allow_yy_calver,
            year_min: cfg.version.year_min,
            year_max: cfg.version.year_max,
        },
    });
    Ok(ListJson {
        schema: "bdg.list/v1".to_string(),
        repo,
        config: config_json,
        readme: readme_json,
        manifests,
        registries,
        ci,
        readme_block,
        warnings: Vec::new(),
    })
}

fn collect_manifests(
    context: &ProjectContext,
    options: &VersionOptions,
) -> anyhow::Result<HashMap<String, serde_json::Value>> {
    let mut manifests = HashMap::new();
    if let Some(path) = &context.manifests.package_json {
        let pkg = read_package_json(path)?;
        let repo = repository_to_string(pkg.repository);
        let version_info = pkg
            .version
            .as_deref()
            .map(|v| crate::version::classify_version(v, options));
        manifests.insert(
            "node".to_string(),
            serde_json::json!({
                "path": path.to_string_lossy(),
                "name": pkg.name,
                "version": pkg.version,
                "version_format": version_info.as_ref().map(|v| v.version_format.clone()),
                "calver_scheme": version_info.as_ref().and_then(|v| v.calver_scheme.clone()),
                "calver_parts": version_info.as_ref().and_then(|v| v.calver_parts.clone()),
                "modifier": version_info.as_ref().and_then(|v| v.modifier.clone()),
                "license": pkg.license,
                "repository": repo,
            }),
        );
    }
    if let Some(path) = &context.manifests.cargo_toml {
        let manifest = read_cargo_toml(path)?;
        if let Some(package) = manifest.package {
            let version_info = package
                .version
                .as_deref()
                .map(|v| crate::version::classify_version(v, options));
            manifests.insert(
                "rust".to_string(),
                serde_json::json!({
                    "path": path.to_string_lossy(),
                    "name": package.name,
                    "version": package.version,
                    "version_format": version_info.as_ref().map(|v| v.version_format.clone()),
                    "calver_scheme": version_info.as_ref().and_then(|v| v.calver_scheme.clone()),
                    "calver_parts": version_info.as_ref().and_then(|v| v.calver_parts.clone()),
                    "modifier": version_info.as_ref().and_then(|v| v.modifier.clone()),
                    "license": package.license,
                    "repository": package.repository,
                }),
            );
        }
    }
    if let Some(path) = &context.manifests.moon_mod {
        let module = read_moon_mod(path)?;
        let version_info = module
            .version
            .as_deref()
            .map(|v| crate::version::classify_version(v, options));
        manifests.insert(
            "moon".to_string(),
            serde_json::json!({
                "path": path.to_string_lossy(),
                "name": module.name,
                "version": module.version,
                "version_format": version_info.as_ref().map(|v| v.version_format.clone()),
                "calver_scheme": version_info.as_ref().and_then(|v| v.calver_scheme.clone()),
                "calver_parts": version_info.as_ref().and_then(|v| v.calver_parts.clone()),
                "modifier": version_info.as_ref().and_then(|v| v.modifier.clone()),
                "readme": module.readme,
            }),
        );
    }
    Ok(manifests)
}

fn collect_registries(
    context: &ProjectContext,
    options: &VersionOptions,
) -> anyhow::Result<HashMap<String, serde_json::Value>> {
    let mut registries = HashMap::new();
    if let Some(path) = &context.manifests.package_json {
        let pkg = read_package_json(path)?;
        if let Some(name) = pkg.name.as_deref() {
            match fetch_npm_metadata(name) {
                Ok(meta) => {
                    let version_info = meta
                        .version
                        .as_deref()
                        .map(|v| crate::version::classify_version(v, options));
                    registries.insert(
                        "npm".to_string(),
                        serde_json::json!({
                            "ok": true,
                            "package": name,
                            "latest": meta.version,
                            "version_format": version_info.as_ref().map(|v| v.version_format.clone()),
                            "calver_scheme": version_info.as_ref().and_then(|v| v.calver_scheme.clone()),
                            "calver_parts": version_info.as_ref().and_then(|v| v.calver_parts.clone()),
                            "modifier": version_info.as_ref().and_then(|v| v.modifier.clone()),
                            "license": meta.license,
                            "homepage": meta.homepage,
                            "repository": meta.repository,
                        }),
                    );
                }
                Err(_) => {
                    registries.insert(
                        "npm".to_string(),
                        serde_json::json!({
                            "ok": false,
                            "package": name,
                            "reason": "network",
                        }),
                    );
                }
            }
        }
    }
    if let Some(path) = &context.manifests.cargo_toml {
        let manifest = read_cargo_toml(path)?;
        if let Some(package) = manifest.package {
            if let Some(name) = package.name.as_deref() {
                match fetch_crates_metadata(name) {
                    Ok(meta) => {
                        let version_info = meta
                            .version
                            .as_deref()
                            .map(|v| crate::version::classify_version(v, options));
                        registries.insert(
                        "crates".to_string(),
                        serde_json::json!({
                            "ok": true,
                            "crate": name,
                            "latest": meta.version,
                            "version_format": version_info.as_ref().map(|v| v.version_format.clone()),
                            "calver_scheme": version_info.as_ref().and_then(|v| v.calver_scheme.clone()),
                            "calver_parts": version_info.as_ref().and_then(|v| v.calver_parts.clone()),
                            "modifier": version_info.as_ref().and_then(|v| v.modifier.clone()),
                            "license": meta.license,
                            "repository": meta.repository,
                            "downloads": meta.downloads,
                        }),
                    );
                    }
                    Err(_) => {
                        registries.insert(
                            "crates".to_string(),
                            serde_json::json!({
                                "ok": false,
                                "crate": name,
                                "reason": "network",
                            }),
                        );
                    }
                }
            }
        }
    }
    if let Some(path) = &context.manifests.moon_mod {
        let module = read_moon_mod(path)?;
        registries.insert(
            "mooncakes".to_string(),
            serde_json::json!({
                "ok": false,
                "module": module.name,
                "reason": "disabled",
            }),
        );
    }
    Ok(registries)
}

fn build_ci_json(context: &ProjectContext) -> anyhow::Result<CiJson> {
    let workflows = detect_workflows(&context.root);
    let workflows_json = workflows
        .iter()
        .map(|wf| workflow_to_json(context, wf))
        .collect::<Vec<_>>();
    Ok(CiJson {
        workflows_dir: ".github/workflows".to_string(),
        workflows: workflows_json,
    })
}

fn workflow_to_json(context: &ProjectContext, workflow: &WorkflowInfo) -> WorkflowJson {
    let mut image = String::new();
    let mut link = String::new();
    if let Some(git) = &context.git {
        if let (Some(owner), Some(repo)) = (git.owner.as_deref(), git.repo.as_deref()) {
            image = format!(
                "https://github.com/{}/{}/actions/workflows/{}/badge.svg",
                owner, repo, workflow.file
            );
            link = format!(
                "https://github.com/{}/{}/actions/workflows/{}",
                owner, repo, workflow.file
            );
        }
    }
    let status = gh_latest_status_json(&workflow.file);
    WorkflowJson {
        file: workflow.file.clone(),
        name: workflow.name.clone(),
        badge: WorkflowBadgeJson {
            kind: "github_actions".to_string(),
            image,
            link,
        },
        latest_status: GhStatusJson {
            source: "gh".to_string(),
            ok: status.ok,
            reason: status.reason,
            conclusion: status.conclusion,
            run_id: status.run_id,
            html_url: status.html_url,
            updated_at: status.updated_at,
        },
    }
}

fn build_readme_block(badges: &[String]) -> ReadmeBlockJson {
    let raw = if badges.is_empty() {
        String::new()
    } else {
        let mut joined = badges.join("\n");
        joined.push('\n');
        joined
    };
    let mut parsed = Vec::new();
    let mut in_code_fence = false;
    for line in badges {
        if is_code_fence(line) {
            in_code_fence = !in_code_fence;
            continue;
        }
        if in_code_fence {
            continue;
        }
        let badge = crate::readme_badges::parse_badge_line(line);
        parsed.push(readme_badge_from_parsed(badge));
    }
    ReadmeBlockJson {
        raw,
        badges: parsed,
    }
}

fn is_code_fence(line: &str) -> bool {
    line.trim_start().starts_with("```")
}

fn readme_badge_from_parsed(parsed: ParsedBadge) -> ReadmeBadgeJson {
    ReadmeBadgeJson {
        id: parsed.id,
        kind: parsed.kind,
        label: parsed.label,
        image: parsed.image,
        link: parsed.link,
        source: parsed.source,
        meta: parsed.meta,
        raw: parsed.raw,
    }
}
