use crate::config::load_config;
use crate::core::build_context;
use crate::manifest::{read_moon_mod, read_package_json, read_resolved_cargo_package};
use crate::plan::ReadmePlan;
use crate::readme::{
    ensure_marker_block, extract_managed_block, resolve_readme, rewrite_marker_block,
};
use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::Duration;

const BUILTIN_CATALOG: &str = include_str!("../catalog/builtin.toml");
const USER_AGENT: &str = concat!("bdg/", env!("CARGO_PKG_VERSION"));

#[derive(Debug, Clone, Deserialize)]
pub struct CatalogFile {
    #[serde(default = "catalog_schema")]
    pub schema: String,
    #[serde(default, rename = "badge", alias = "badges")]
    pub badges: Vec<CatalogBadge>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CatalogBadge {
    pub id: String,
    #[serde(default = "default_kind")]
    pub kind: String,
    pub label: String,
    pub image: String,
    #[serde(default)]
    pub link: Option<String>,
    #[serde(default)]
    pub requires: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
struct LoadedBadge {
    badge: CatalogBadge,
    source: String,
}

#[derive(Debug, Serialize)]
struct SearchPayload {
    schema: &'static str,
    query: Option<String>,
    results: Vec<SearchResult>,
}

#[derive(Debug, Serialize)]
struct SearchResult {
    id: String,
    kind: String,
    label: String,
    description: Option<String>,
    tags: Vec<String>,
    source: String,
    available: bool,
    missing: Vec<String>,
}

#[derive(Debug, Serialize)]
struct AddPayload {
    schema: &'static str,
    path: String,
    added: Vec<String>,
    unchanged: Vec<String>,
    diff: String,
}

pub fn cmd_catalog_search(
    current_dir: &Path,
    query: Option<&str>,
    sources: &[String],
    json: bool,
) -> anyhow::Result<()> {
    let catalog = load_catalog(current_dir, sources)?;
    let values = project_values(current_dir)?;
    let needle = query.map(|value| value.trim().to_lowercase());
    let mut results = Vec::new();

    for loaded in catalog.values() {
        if let Some(needle) = needle.as_deref()
            && !needle.is_empty()
            && !matches_query(&loaded.badge, needle)
        {
            continue;
        }
        let missing = missing_values(&loaded.badge, &values);
        results.push(SearchResult {
            id: loaded.badge.id.clone(),
            kind: loaded.badge.kind.clone(),
            label: loaded.badge.label.clone(),
            description: loaded.badge.description.clone(),
            tags: loaded.badge.tags.clone(),
            source: loaded.source.clone(),
            available: missing.is_empty(),
            missing,
        });
    }

    if json {
        serde_json::to_writer_pretty(
            std::io::stdout(),
            &SearchPayload {
                schema: "bdg.catalog.search/v1",
                query: query.map(str::to_string),
                results,
            },
        )?;
        println!();
        return Ok(());
    }

    for result in results {
        let availability = if result.available {
            "ready".to_string()
        } else {
            format!("needs {}", result.missing.join(","))
        };
        let description = result
            .description
            .as_deref()
            .map(|text| format!(" — {text}"))
            .unwrap_or_default();
        println!(
            "{} [{}] {} ({}){}",
            result.id, result.kind, result.label, availability, description
        );
    }
    Ok(())
}

pub fn cmd_catalog_add(
    current_dir: &Path,
    ids: &[String],
    sources: &[String],
    set_values: &[String],
    dry_run: bool,
    json: bool,
) -> anyhow::Result<i32> {
    if ids.is_empty() {
        bail!("catalog add requires at least one badge id");
    }

    let catalog = load_catalog(current_dir, sources)?;
    let mut values = project_values(current_dir)?;
    for assignment in set_values {
        let (key, value) = assignment
            .split_once('=')
            .with_context(|| format!("invalid --set `{assignment}`; expected KEY=VALUE"))?;
        let key = key.trim();
        if key.is_empty() || value.is_empty() {
            bail!("invalid --set `{assignment}`; expected non-empty KEY=VALUE");
        }
        values.insert(key.to_string(), value.to_string());
    }

    let mut rendered = Vec::new();
    for id in ids {
        let loaded = catalog
            .get(id)
            .with_context(|| format!("badge `{id}` not found in catalog"))?;
        let missing = missing_values(&loaded.badge, &values);
        if !missing.is_empty() {
            bail!(
                "badge `{id}` requires {}; provide project metadata or --set KEY=VALUE",
                missing.join(", ")
            );
        }
        rendered.push((id.clone(), render_badge(&loaded.badge, &values)?));
    }
    apply_rendered_badges(current_dir, rendered, dry_run, json)
}

pub fn cmd_catalog_add_url(
    current_dir: &Path,
    image: &str,
    label: &str,
    link: Option<&str>,
    dry_run: bool,
    json: bool,
) -> anyhow::Result<i32> {
    if !image.starts_with("https://") && !image.starts_with("http://") {
        bail!("catalog add-url IMAGE_URL must be HTTP(S)");
    }
    if label.trim().is_empty() {
        bail!("catalog add-url --label must not be empty");
    }
    let markdown = match link {
        Some(link) => format!("[![{label}]({image})]({link})"),
        None => format!("![{label}]({image})"),
    };
    apply_rendered_badges(
        current_dir,
        vec![("external-url".to_string(), markdown)],
        dry_run,
        json,
    )
}

fn apply_rendered_badges(
    current_dir: &Path,
    rendered: Vec<(String, String)>,
    dry_run: bool,
    json: bool,
) -> anyhow::Result<i32> {
    let context = build_context(current_dir)?;
    let readme_path = resolve_readme(&context.root, context.has_moonbit());
    let content = ensure_marker_block(&readme_path)?;
    let mut lines = extract_managed_block(&content);
    let mut existing_images = lines
        .iter()
        .filter_map(|line| crate::readme_badges::parse_badge_line_optional(line))
        .map(|badge| badge.image)
        .collect::<BTreeSet<_>>();
    let mut added = Vec::new();
    let mut unchanged = Vec::new();

    for (id, markdown) in rendered {
        let parsed = crate::readme_badges::parse_badge_line_optional(&markdown)
            .context("catalog rendered invalid badge Markdown")?;
        if existing_images.insert(parsed.image) {
            lines.push(markdown);
            added.push(id);
        } else {
            unchanged.push(id);
        }
    }

    let updated = rewrite_marker_block(&content, &lines)?;
    let plan = ReadmePlan::new(readme_path, content, updated);
    let diff = plan.diff();

    if json {
        serde_json::to_writer_pretty(
            std::io::stdout(),
            &AddPayload {
                schema: "bdg.catalog.add/v1",
                path: plan.path().to_string_lossy().to_string(),
                added,
                unchanged,
                diff: diff.clone(),
            },
        )?;
        println!();
    } else if dry_run && !diff.is_empty() {
        print!("{diff}");
    } else if !dry_run {
        for id in &added {
            println!("added {id}");
        }
        for id in &unchanged {
            println!("unchanged {id}");
        }
    }

    if dry_run {
        return Ok(if diff.is_empty() { 0 } else { 2 });
    }
    plan.apply()?;
    Ok(0)
}

fn load_catalog(
    current_dir: &Path,
    sources: &[String],
) -> anyhow::Result<BTreeMap<String, LoadedBadge>> {
    let mut catalog = BTreeMap::new();
    merge_catalog(&mut catalog, parse_catalog(BUILTIN_CATALOG)?, "builtin");

    let context = build_context(current_dir)?;
    let local = context.root.join(".bdg/catalog.toml");
    if local.is_file() {
        let text = std::fs::read_to_string(&local)?;
        merge_catalog(
            &mut catalog,
            parse_catalog(&text)?,
            &local.to_string_lossy(),
        );
    }

    let config = load_config(current_dir, &context.root)?;
    for source in config.catalog.sources {
        let (text, source_name) = read_source(&context.root, &source)?;
        merge_catalog(&mut catalog, parse_catalog(&text)?, &source_name);
    }
    for source in sources {
        let (text, source_name) = read_source(current_dir, source)?;
        merge_catalog(&mut catalog, parse_catalog(&text)?, &source_name);
    }
    Ok(catalog)
}

fn merge_catalog(
    target: &mut BTreeMap<String, LoadedBadge>,
    source: CatalogFile,
    source_name: &str,
) {
    for badge in source.badges {
        target.insert(
            badge.id.clone(),
            LoadedBadge {
                badge,
                source: source_name.to_string(),
            },
        );
    }
}

fn read_source(current_dir: &Path, source: &str) -> anyhow::Result<(String, String)> {
    if source.starts_with("https://") || source.starts_with("http://") {
        let config = ureq::Agent::config_builder()
            .timeout_connect(Some(Duration::from_secs(3)))
            .timeout_global(Some(Duration::from_secs(8)))
            .build();
        let agent = ureq::Agent::new_with_config(config);
        let mut response = agent
            .get(source)
            .header("User-Agent", USER_AGENT)
            .header("Accept", "application/toml, application/json, text/plain")
            .call()
            .with_context(|| format!("fetch catalog {source}"))?;
        return Ok((response.body_mut().read_to_string()?, source.to_string()));
    }

    let path = PathBuf::from(source);
    let path = if path.is_absolute() {
        path
    } else {
        current_dir.join(path)
    };
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("read catalog {}", path.display()))?;
    Ok((text, path.to_string_lossy().to_string()))
}

fn parse_catalog(text: &str) -> anyhow::Result<CatalogFile> {
    let trimmed = text.trim_start();
    let catalog = if trimmed.starts_with('{') {
        serde_json::from_str(text).context("parse catalog JSON")?
    } else {
        toml::from_str(text).context("parse catalog TOML")?
    };
    validate_catalog(&catalog)?;
    Ok(catalog)
}

fn validate_catalog(catalog: &CatalogFile) -> anyhow::Result<()> {
    if catalog.schema != "bdg.catalog/v1" {
        bail!(
            "unsupported catalog schema `{}`; expected bdg.catalog/v1",
            catalog.schema
        );
    }
    let mut ids = BTreeSet::new();
    for badge in &catalog.badges {
        if badge.id.trim().is_empty() {
            bail!("catalog badge id must not be empty");
        }
        if !ids.insert(badge.id.as_str()) {
            bail!("duplicate catalog badge id `{}`", badge.id);
        }
        if badge.label.trim().is_empty() || badge.image.trim().is_empty() {
            bail!("catalog badge `{}` requires label and image", badge.id);
        }
        if !badge.image.starts_with("https://") && !badge.image.starts_with("http://") {
            bail!("catalog badge `{}` image must be an HTTP(S) URL", badge.id);
        }
    }
    Ok(())
}

fn matches_query(badge: &CatalogBadge, query: &str) -> bool {
    badge.id.to_lowercase().contains(query)
        || badge.kind.to_lowercase().contains(query)
        || badge.label.to_lowercase().contains(query)
        || badge
            .description
            .as_deref()
            .is_some_and(|value| value.to_lowercase().contains(query))
        || badge
            .tags
            .iter()
            .any(|tag| tag.to_lowercase().contains(query))
}

fn project_values(current_dir: &Path) -> anyhow::Result<BTreeMap<String, String>> {
    let context = build_context(current_dir)?;
    let mut values = BTreeMap::new();

    if let Some(git) = &context.git {
        if let Some(owner) = &git.owner {
            values.insert("owner".to_string(), owner.clone());
        }
        if let Some(repo) = &git.repo {
            values.insert("repo".to_string(), repo.clone());
        }
    }
    if let Some(path) = &context.manifests.cargo_toml
        && let Some(package) = read_resolved_cargo_package(path)?
        && let Some(name) = package.name
    {
        values.insert("crate".to_string(), name.clone());
        values.entry("name".to_string()).or_insert(name);
    }
    if let Some(path) = &context.manifests.package_json
        && let Ok(package) = read_package_json(path)
        && let Some(name) = package.name
    {
        values.insert("package".to_string(), name.clone());
        values.entry("name".to_string()).or_insert(name);
    }
    if let Some(path) = &context.manifests.moon_mod
        && let Ok(module) = read_moon_mod(path)
        && let Some(name) = module.name
    {
        values.insert("module".to_string(), name.clone());
        values.entry("name".to_string()).or_insert(name);
    }
    Ok(values)
}

fn missing_values(badge: &CatalogBadge, values: &BTreeMap<String, String>) -> Vec<String> {
    let mut required = badge.requires.iter().cloned().collect::<BTreeSet<_>>();
    for template in [&badge.label, &badge.image] {
        required.extend(placeholders(template));
    }
    if let Some(link) = &badge.link {
        required.extend(placeholders(link));
    }
    required
        .into_iter()
        .filter(|key| !values.contains_key(key))
        .collect()
}

fn placeholders(template: &str) -> BTreeSet<String> {
    let mut output = BTreeSet::new();
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        let after = &rest[start + 1..];
        let Some(end) = after.find('}') else {
            break;
        };
        let key = &after[..end];
        if !key.is_empty()
            && key
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
        {
            output.insert(key.to_string());
        }
        rest = &after[end + 1..];
    }
    output
}

fn render_badge(badge: &CatalogBadge, values: &BTreeMap<String, String>) -> anyhow::Result<String> {
    let label = render_template(&badge.label, values)?;
    let image = render_template(&badge.image, values)?;
    let link = badge
        .link
        .as_deref()
        .map(|template| render_template(template, values))
        .transpose()?;
    Ok(match link {
        Some(link) => format!("[![{label}]({image})]({link})"),
        None => format!("![{label}]({image})"),
    })
}

fn render_template(template: &str, values: &BTreeMap<String, String>) -> anyhow::Result<String> {
    let mut rendered = template.to_string();
    for key in placeholders(template) {
        let value = values
            .get(&key)
            .with_context(|| format!("missing template value `{key}`"))?;
        rendered = rendered.replace(&format!("{{{key}}}"), value);
    }
    Ok(rendered)
}

fn catalog_schema() -> String {
    "bdg.catalog/v1".to_string()
}

fn default_kind() -> String {
    "external".to_string()
}

#[cfg(test)]
mod tests {
    use super::{parse_catalog, placeholders, render_template};
    use std::collections::BTreeMap;

    #[test]
    fn parses_toml_and_json_catalogs() {
        let toml = r#"
schema = "bdg.catalog/v1"
[[badge]]
id = "demo"
label = "demo"
image = "https://img.shields.io/badge/demo-{value}-blue"
requires = ["value"]
"#;
        assert_eq!(parse_catalog(toml).unwrap().badges.len(), 1);

        let json = r#"{
          "schema": "bdg.catalog/v1",
          "badges": [{"id":"demo","label":"demo","image":"https://example.com/badge.svg"}]
        }"#;
        assert_eq!(parse_catalog(json).unwrap().badges.len(), 1);
    }

    #[test]
    fn expands_declared_placeholders() {
        assert_eq!(
            placeholders("https://x/{owner}/{repo}/{owner}"),
            ["owner".to_string(), "repo".to_string()]
                .into_iter()
                .collect()
        );
        let values = BTreeMap::from([
            ("owner".to_string(), "f4ah6o".to_string()),
            ("repo".to_string(), "bdg-rs".to_string()),
        ]);
        assert_eq!(
            render_template("{owner}/{repo}", &values).unwrap(),
            "f4ah6o/bdg-rs"
        );
    }

    #[test]
    fn rejects_duplicate_ids() {
        let catalog = r#"
schema = "bdg.catalog/v1"
[[badge]]
id = "same"
label = "one"
image = "https://example.com/one.svg"
[[badge]]
id = "same"
label = "two"
image = "https://example.com/two.svg"
"#;
        assert!(parse_catalog(catalog).is_err());
    }
}
