use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct RegistryMetadata {
    pub version: Option<String>,
    pub license: Option<String>,
    pub repository: Option<String>,
    pub description: Option<String>,
    pub downloads: Option<u64>,
    pub homepage: Option<String>,
}

impl RegistryMetadata {
    pub fn empty() -> Self {
        Self {
            version: None,
            license: None,
            repository: None,
            description: None,
            downloads: None,
            homepage: None,
        }
    }
}

pub fn fetch_npm_metadata(package: &str) -> anyhow::Result<RegistryMetadata> {
    let url = format!("https://registry.npmjs.org/{}", package);
    let client = reqwest::blocking::Client::new();
    let response = client.get(url).send()?;
    let payload: NpmPackument = response.json()?;
    let version = payload
        .dist_tags
        .get("latest")
        .cloned()
        .or_else(|| payload.version);
    Ok(RegistryMetadata {
        version,
        license: payload.license,
        repository: payload.repository.and_then(|repo| repo.url),
        description: payload.description,
        downloads: None,
        homepage: payload.homepage,
    })
}

pub fn fetch_crates_metadata(crate_name: &str) -> anyhow::Result<RegistryMetadata> {
    let url = format!("https://crates.io/api/v1/crates/{}", crate_name);
    let client = reqwest::blocking::Client::new();
    let response = client.get(url).send()?;
    let payload: CratesResponse = response.json()?;
    Ok(RegistryMetadata {
        version: Some(payload.crate_data.max_version),
        license: payload.crate_data.license,
        repository: payload.crate_data.repository,
        description: payload.crate_data.description,
        downloads: Some(payload.crate_data.downloads as u64),
        homepage: payload.crate_data.homepage,
    })
}

#[derive(Debug, Deserialize)]
struct NpmPackument {
    #[serde(rename = "dist-tags")]
    dist_tags: std::collections::HashMap<String, String>,
    version: Option<String>,
    license: Option<String>,
    repository: Option<NpmRepository>,
    description: Option<String>,
    homepage: Option<String>,
}

#[derive(Debug, Deserialize)]
struct NpmRepository {
    url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CratesResponse {
    #[serde(rename = "crate")]
    crate_data: CrateData,
}

#[derive(Debug, Deserialize)]
struct CrateData {
    max_version: String,
    license: Option<String>,
    repository: Option<String>,
    description: Option<String>,
    downloads: u64,
    homepage: Option<String>,
}
