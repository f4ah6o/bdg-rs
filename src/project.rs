use crate::core::{Ecosystem, ProjectContext};
use crate::manifest::{
    RepositoryField, read_moon_mod, read_package_json, read_resolved_cargo_package,
};
use crate::providers::{RegistryMetadata, fetch_crates_metadata, fetch_npm_metadata};
use anyhow::Context;
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
pub struct ResolvedMetadata {
    pub name: Option<String>,
    pub version: Option<String>,
    pub license: Option<String>,
    pub repository: Option<String>,
    pub description: Option<String>,
    pub registry: Option<RegistryMetadata>,
}

#[derive(Debug, Clone)]
pub(crate) struct NpmPackage {
    pub(crate) path: PathBuf,
    pub(crate) name: String,
    pub(crate) version: Option<String>,
    pub(crate) license: Option<String>,
    pub(crate) repository: Option<RepositoryField>,
    pub(crate) description: Option<String>,
    pub(crate) private: bool,
    pub(crate) registry: RegistryMetadata,
    pub(crate) published: bool,
}

pub(crate) fn local_npm_packages(context: &ProjectContext) -> Vec<NpmPackage> {
    let mut seen = HashSet::new();
    let mut packages = Vec::new();
    for path in &context.manifests.package_json_all {
        if !seen.insert(path.clone()) {
            continue;
        }
        let Ok(pkg) = read_package_json(path) else {
            continue;
        };
        if pkg.private.unwrap_or(false) {
            continue;
        }
        let Some(name) = pkg.name.clone() else {
            continue;
        };
        let registry = fetch_npm_metadata(&name).unwrap_or_else(|_| RegistryMetadata::empty());
        let published = registry.version.is_some();
        packages.push(NpmPackage {
            path: path.clone(),
            name,
            version: pkg.version,
            license: pkg.license,
            repository: pkg.repository,
            description: pkg.description,
            private: false,
            registry,
            published,
        });
    }
    packages.sort_by(|a, b| a.path.cmp(&b.path));
    packages
}

pub(crate) fn select_representative_npm_package(
    packages: &[NpmPackage],
    repo_name: Option<&str>,
) -> Option<NpmPackage> {
    if let Some(repo_name) = repo_name
        && let Some(package) = packages.iter().find(|package| package.name == repo_name)
    {
        return Some(package.clone());
    }
    packages.first().cloned()
}

pub(crate) fn resolve_metadata(
    context: &ProjectContext,
    npm_packages: Option<&[NpmPackage]>,
) -> anyhow::Result<ResolvedMetadata> {
    match context.ecosystem {
        Some(Ecosystem::Node) => resolve_node_metadata(context, npm_packages),
        Some(Ecosystem::MoonBit) => resolve_moonbit_metadata(context),
        Some(Ecosystem::Rust) => resolve_rust_metadata(context),
        None => Ok(ResolvedMetadata::default()),
    }
}

fn resolve_node_metadata(
    context: &ProjectContext,
    npm_packages: Option<&[NpmPackage]>,
) -> anyhow::Result<ResolvedMetadata> {
    let local_packages;
    let packages = match npm_packages {
        Some(packages) => packages,
        None => {
            local_packages = local_npm_packages(context);
            &local_packages
        }
    };
    let repo_name = context.git.as_ref().and_then(|git| git.repo.as_deref());
    let package = select_representative_npm_package(
        &packages
            .iter()
            .filter(|package| package.published)
            .cloned()
            .collect::<Vec<_>>(),
        repo_name,
    )
    .or_else(|| packages.first().cloned())
    .with_context(|| {
        if context.manifests.package_json_all.is_empty() {
            "package.json missing"
        } else {
            "no publishable npm packages found"
        }
    })?;
    Ok(ResolvedMetadata {
        name: Some(package.name.clone()),
        version: package.registry.version.clone().or(package.version),
        license: package.registry.license.clone().or(package.license),
        repository: package
            .registry
            .repository
            .clone()
            .or_else(|| repository_to_string(package.repository)),
        description: package.registry.description.clone().or(package.description),
        registry: Some(package.registry),
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
    let package = read_resolved_cargo_package(manifest_path)?.unwrap_or_default();
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

pub(crate) fn infer_owner_repo(repository: &Option<String>) -> (Option<String>, Option<String>) {
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

pub(crate) fn repository_to_string(repo: Option<RepositoryField>) -> Option<String> {
    match repo {
        Some(RepositoryField::String(value)) => Some(value),
        Some(RepositoryField::Object { url }) => url,
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{NpmPackage, select_representative_npm_package};
    use crate::providers::RegistryMetadata;
    use std::path::PathBuf;

    fn npm_package(path: &str, name: &str) -> NpmPackage {
        NpmPackage {
            path: PathBuf::from(path),
            name: name.to_string(),
            version: Some("1.0.0".to_string()),
            license: None,
            repository: None,
            description: None,
            private: false,
            registry: RegistryMetadata {
                version: Some("1.0.0".to_string()),
                license: None,
                repository: None,
                description: None,
                downloads: None,
                homepage: None,
            },
            published: true,
        }
    }

    #[test]
    fn representative_npm_package_prefers_repo_name() {
        let selected = select_representative_npm_package(
            &[
                npm_package("packages/core/package.json", "n8n-core"),
                npm_package("packages/cli/package.json", "n8n"),
            ],
            Some("n8n"),
        )
        .expect("selected");

        assert_eq!(selected.name, "n8n");
    }

    #[test]
    fn representative_npm_package_falls_back_to_sorted_first() {
        let selected = select_representative_npm_package(
            &[
                npm_package("packages/cli/package.json", "n8n"),
                npm_package("packages/core/package.json", "n8n-core"),
            ],
            Some("other"),
        )
        .expect("selected");

        assert_eq!(selected.name, "n8n");
    }
}
