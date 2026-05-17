use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct PackageJson {
    pub name: Option<String>,
    pub version: Option<String>,
    pub private: Option<bool>,
    pub description: Option<String>,
    pub license: Option<String>,
    pub repository: Option<RepositoryField>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum RepositoryField {
    String(String),
    Object { url: Option<String> },
}

#[derive(Debug, Deserialize)]
pub struct MoonMod {
    pub name: Option<String>,
    pub version: Option<String>,
    pub readme: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CargoToml {
    pub package: Option<CargoPackage>,
    pub workspace: Option<CargoWorkspace>,
}

#[derive(Debug, Deserialize)]
pub struct CargoPackage {
    pub name: Option<CargoPackageField>,
    pub version: Option<CargoPackageField>,
    pub description: Option<CargoPackageField>,
    pub license: Option<CargoPackageField>,
    pub repository: Option<CargoPackageField>,
}

#[derive(Debug, Deserialize)]
pub struct CargoWorkspace {
    pub package: Option<CargoWorkspacePackage>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CargoWorkspacePackage {
    pub version: Option<String>,
    pub description: Option<String>,
    pub license: Option<String>,
    pub repository: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum CargoPackageField {
    Value(String),
    Workspace { workspace: bool },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResolvedCargoPackage {
    pub name: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub license: Option<String>,
    pub repository: Option<String>,
}

impl CargoPackageField {
    fn local_value(&self) -> Option<String> {
        match self {
            CargoPackageField::Value(value) => Some(value.clone()),
            CargoPackageField::Workspace { .. } => None,
        }
    }

    fn inherits_workspace(&self) -> bool {
        matches!(self, CargoPackageField::Workspace { workspace: true })
    }
}

pub fn read_package_json(path: &Path) -> anyhow::Result<PackageJson> {
    let content = std::fs::read_to_string(path)?;
    let package: PackageJson = serde_json::from_str(&content)?;
    Ok(package)
}

pub fn read_moon_mod(path: &Path) -> anyhow::Result<MoonMod> {
    let content = std::fs::read_to_string(path)?;
    let module: MoonMod = serde_json::from_str(&content)?;
    Ok(module)
}

pub fn read_cargo_toml(path: &Path) -> anyhow::Result<CargoToml> {
    let content = std::fs::read_to_string(path)?;
    let manifest: CargoToml = toml::from_str(&content)?;
    Ok(manifest)
}

pub fn read_resolved_cargo_package(path: &Path) -> anyhow::Result<Option<ResolvedCargoPackage>> {
    let manifest = read_cargo_toml(path)?;
    let Some(package) = manifest.package else {
        return Ok(None);
    };
    let workspace_package = match manifest.workspace.and_then(|workspace| workspace.package) {
        Some(package) => Some(package),
        None => find_workspace_package(path)?,
    };
    Ok(Some(resolve_cargo_package(
        package,
        workspace_package.as_ref(),
    )))
}

pub fn cargo_manifest_has_package(path: &Path) -> bool {
    read_cargo_toml(path)
        .map(|manifest| manifest.package.is_some())
        .unwrap_or(false)
}

fn resolve_cargo_package(
    package: CargoPackage,
    workspace_package: Option<&CargoWorkspacePackage>,
) -> ResolvedCargoPackage {
    ResolvedCargoPackage {
        name: resolve_field(package.name, None),
        version: resolve_field(
            package.version,
            workspace_package.and_then(|package| package.version.clone()),
        ),
        description: resolve_field(
            package.description,
            workspace_package.and_then(|package| package.description.clone()),
        ),
        license: resolve_field(
            package.license,
            workspace_package.and_then(|package| package.license.clone()),
        ),
        repository: resolve_field(
            package.repository,
            workspace_package.and_then(|package| package.repository.clone()),
        ),
    }
}

fn resolve_field(
    field: Option<CargoPackageField>,
    workspace_value: Option<String>,
) -> Option<String> {
    match field {
        Some(field) if field.inherits_workspace() => workspace_value,
        Some(field) => field.local_value(),
        None => None,
    }
}

fn find_workspace_package(path: &Path) -> anyhow::Result<Option<CargoWorkspacePackage>> {
    let mut dir = path.parent().map(Path::to_path_buf);
    while let Some(current) = dir {
        let candidate = current.join("Cargo.toml");
        if candidate != path && candidate.exists() {
            let manifest = read_cargo_toml(&candidate)?;
            if let Some(workspace) = manifest.workspace {
                return Ok(workspace.package);
            }
        }
        dir = current.parent().map(Path::to_path_buf);
    }
    Ok(None)
}
