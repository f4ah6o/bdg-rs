use crate::config::Config;
use crate::core::ProjectContext;
use crate::manifest::{read_moon_mod, read_package_json, read_resolved_cargo_package};
use crate::project::{
    NpmPackage, local_npm_packages, repository_to_string, select_representative_npm_package,
};
use crate::providers::{RegistryMetadata, fetch_crates_metadata};
use crate::readme::readme_newline_info;
use crate::readme_badges::ParsedBadge;
use crate::version::VersionOptions;
use crate::workflows::{WorkflowInfo, detect_workflows, gh_latest_status_json_in};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Serialize)]
pub(crate) struct ListJson {
    schema: String,
    repo: Option<RepoJson>,
    config: Option<ConfigJson>,
    readme: ReadmeJson,
    manifests: HashMap<String, serde_json::Value>,
    registries: HashMap<String, serde_json::Value>,
    ci: CiJson,
    readme_block: ReadmeBlockJson,
    warnings: Vec<ListWarningJson>,
}

#[derive(Debug, Serialize)]
struct ConfigJson {
    version: ConfigVersionJson,
    badges: ConfigBadgesJson,
    catalog: ConfigCatalogJson,
}

#[derive(Debug, Serialize)]
struct ConfigVersionJson {
    allow_yy_calver: bool,
    year_min: i32,
    year_max: i32,
}

#[derive(Debug, Serialize)]
struct ConfigBadgesJson {
    exclude: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ConfigCatalogJson {
    sources: Vec<String>,
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
    begin_count: usize,
    end_count: usize,
    ordered: bool,
    valid: bool,
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
struct ListWarningJson {
    code: String,
    message: String,
    meta: Option<serde_json::Value>,
}

pub(crate) fn build_list_json(
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
    let marker = crate::readme::marker_state(content);
    let marker_count = marker.begin_count;
    let readme_json = ReadmeJson {
        path: readme_path.to_string_lossy().to_string(),
        newline,
        trailing_newline: trailing,
        markers: MarkerJson {
            present: marker.is_valid(),
            count: marker_count,
            begin_count: marker.begin_count,
            end_count: marker.end_count,
            ordered: marker.ordered,
            valid: marker.is_valid(),
        },
    };

    let npm_packages = local_npm_packages(context);
    let manifests = collect_manifests(context, options, &npm_packages)?;
    let registries = collect_registries(context, options, &npm_packages)?;
    let ci = build_ci_json(context)?;
    let readme_block = build_readme_block(badges);

    let config_json = config.map(|cfg| ConfigJson {
        version: ConfigVersionJson {
            allow_yy_calver: cfg.version.allow_yy_calver,
            year_min: cfg.version.year_min,
            year_max: cfg.version.year_max,
        },
        badges: ConfigBadgesJson {
            exclude: cfg.badges.exclude.clone(),
        },
        catalog: ConfigCatalogJson {
            sources: cfg.catalog.sources.clone(),
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
    node_packages: &[NpmPackage],
) -> anyhow::Result<HashMap<String, serde_json::Value>> {
    let mut manifests = HashMap::new();
    let representative_node = select_representative_npm_package(
        node_packages,
        context.git.as_ref().and_then(|git| git.repo.as_deref()),
    )
    .or_else(|| {
        context
            .manifests
            .package_json
            .as_ref()
            .and_then(|path| read_package_json(path).ok().map(|pkg| (path, pkg)))
            .and_then(|(path, pkg)| {
                Some(NpmPackage {
                    path: path.clone(),
                    name: pkg.name?,
                    version: pkg.version,
                    license: pkg.license,
                    repository: pkg.repository,
                    description: pkg.description,
                    private: pkg.private.unwrap_or(false),
                    registry: RegistryMetadata::empty(),
                    published: false,
                })
            })
    });
    if let Some(pkg) = representative_node {
        let repo = repository_to_string(pkg.repository);
        let version_info = pkg
            .version
            .as_deref()
            .map(|v| crate::version::classify_version(v, options));
        manifests.insert(
            "node".to_string(),
            serde_json::json!({
                "path": pkg.path.to_string_lossy(),
                "name": pkg.name,
                "version": pkg.version,
                "version_format": version_info.as_ref().map(|v| v.version_format.clone()),
                "calver_scheme": version_info.as_ref().and_then(|v| v.calver_scheme.clone()),
                "calver_parts": version_info.as_ref().and_then(|v| v.calver_parts.clone()),
                "modifier": version_info.as_ref().and_then(|v| v.modifier.clone()),
                "license": pkg.license,
                "repository": repo,
                "private": pkg.private,
                "published": pkg.published,
            }),
        );
    }
    if !node_packages.is_empty() {
        let packages = node_packages
            .iter()
            .map(|pkg| {
                let version_info = pkg
                    .version
                    .as_deref()
                    .map(|v| crate::version::classify_version(v, options));
                serde_json::json!({
                    "path": pkg.path.to_string_lossy(),
                    "name": pkg.name,
                    "version": pkg.version,
                    "version_format": version_info.as_ref().map(|v| v.version_format.clone()),
                    "calver_scheme": version_info.as_ref().and_then(|v| v.calver_scheme.clone()),
                    "calver_parts": version_info.as_ref().and_then(|v| v.calver_parts.clone()),
                    "modifier": version_info.as_ref().and_then(|v| v.modifier.clone()),
                    "license": pkg.license,
                    "repository": repository_to_string(pkg.repository.clone()),
                    "private": pkg.private,
                    "published": pkg.published,
                })
            })
            .collect::<Vec<_>>();
        manifests.insert(
            "node_packages".to_string(),
            serde_json::Value::Array(packages),
        );
    }
    if let Some(path) = &context.manifests.cargo_toml
        && let Some(package) = read_resolved_cargo_package(path)?
    {
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
    npm_packages: &[NpmPackage],
) -> anyhow::Result<HashMap<String, serde_json::Value>> {
    let mut registries = HashMap::new();
    let npm_registry_packages = npm_packages
        .iter()
        .map(|package| {
            let version_info = package
                .registry
                .version
                .as_deref()
                .map(|v| crate::version::classify_version(v, options));
            serde_json::json!({
                "ok": package.published,
                "package": package.name,
                "path": package.path.to_string_lossy(),
                "latest": package.registry.version,
                "version_format": version_info.as_ref().map(|v| v.version_format.clone()),
                "calver_scheme": version_info.as_ref().and_then(|v| v.calver_scheme.clone()),
                "calver_parts": version_info.as_ref().and_then(|v| v.calver_parts.clone()),
                "modifier": version_info.as_ref().and_then(|v| v.modifier.clone()),
                "license": package.registry.license,
                "homepage": package.registry.homepage,
                "repository": package.registry.repository,
                "reason": if package.published { None } else { Some("unavailable") },
            })
        })
        .collect::<Vec<_>>();
    if !npm_registry_packages.is_empty() {
        registries.insert(
            "npm_packages".to_string(),
            serde_json::Value::Array(npm_registry_packages),
        );
    }
    let published_packages = npm_packages
        .iter()
        .filter(|package| package.published)
        .cloned()
        .collect::<Vec<_>>();
    if let Some(package) = select_representative_npm_package(
        &published_packages,
        context.git.as_ref().and_then(|git| git.repo.as_deref()),
    )
    .or_else(|| {
        select_representative_npm_package(
            npm_packages,
            context.git.as_ref().and_then(|git| git.repo.as_deref()),
        )
    }) {
        let version_info = package
            .registry
            .version
            .as_deref()
            .map(|v| crate::version::classify_version(v, options));
        registries.insert(
            "npm".to_string(),
            serde_json::json!({
                "ok": package.published,
                "package": package.name,
                "path": package.path.to_string_lossy(),
                "latest": package.registry.version,
                "version_format": version_info.as_ref().map(|v| v.version_format.clone()),
                "calver_scheme": version_info.as_ref().and_then(|v| v.calver_scheme.clone()),
                "calver_parts": version_info.as_ref().and_then(|v| v.calver_parts.clone()),
                "modifier": version_info.as_ref().and_then(|v| v.modifier.clone()),
                "license": package.registry.license,
                "homepage": package.registry.homepage,
                "repository": package.registry.repository,
                "reason": if package.published { None } else { Some("unavailable") },
            }),
        );
    }
    if let Some(path) = &context.manifests.cargo_toml
        && let Some(package) = read_resolved_cargo_package(path)?
        && let Some(name) = package.name.as_deref()
    {
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
    if let Some(git) = &context.git
        && let (Some(owner), Some(repo)) = (git.owner.as_deref(), git.repo.as_deref())
    {
        image = format!(
            "https://github.com/{}/{}/actions/workflows/{}/badge.svg",
            owner, repo, workflow.file
        );
        link = format!(
            "https://github.com/{}/{}/actions/workflows/{}",
            owner, repo, workflow.file
        );
    }
    let status = gh_latest_status_json_in(&context.root, &workflow.file);
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
