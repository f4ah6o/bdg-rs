use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct PackageJson {
    pub name: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub license: Option<String>,
    pub repository: Option<RepositoryField>,
}

#[derive(Debug, Deserialize)]
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
}

#[derive(Debug, Deserialize)]
pub struct CargoPackage {
    pub name: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub license: Option<String>,
    pub repository: Option<String>,
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
