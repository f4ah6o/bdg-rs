use serde::Deserialize;

const USER_AGENT: &str = concat!("bdg/", env!("CARGO_PKG_VERSION"));

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
    let text = fetch_json_text(&url)?;
    parse_npm_metadata(&text)
}

pub fn fetch_crates_metadata(crate_name: &str) -> anyhow::Result<RegistryMetadata> {
    let url = format!("https://crates.io/api/v1/crates/{}", crate_name);
    let text = fetch_json_text(&url)?;
    parse_crates_metadata(&text)
}

fn parse_npm_metadata(text: &str) -> anyhow::Result<RegistryMetadata> {
    let payload: NpmPackument = serde_json::from_str(text)?;
    let version = payload.dist_tags.get("latest").cloned().or(payload.version);
    Ok(RegistryMetadata {
        version,
        license: payload.license,
        repository: payload.repository.and_then(|repo| repo.url),
        description: payload.description,
        downloads: None,
        homepage: payload.homepage,
    })
}

fn parse_crates_metadata(text: &str) -> anyhow::Result<RegistryMetadata> {
    let payload: CratesResponse = serde_json::from_str(text)?;
    Ok(RegistryMetadata {
        version: Some(payload.crate_data.max_version),
        license: payload.crate_data.license,
        repository: payload.crate_data.repository,
        description: payload.crate_data.description,
        downloads: Some(payload.crate_data.downloads as u64),
        homepage: payload.crate_data.homepage,
    })
}

fn fetch_json_text(url: &str) -> anyhow::Result<String> {
    let mut response = ureq::get(url)
        .header("User-Agent", USER_AGENT)
        .header("Accept", "application/json")
        .call()?;
    Ok(response.body_mut().read_to_string()?)
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

#[cfg(test)]
mod tests {
    use super::{USER_AGENT, parse_crates_metadata, parse_npm_metadata};

    #[test]
    fn user_agent_includes_crate_version() {
        assert_eq!(USER_AGENT, concat!("bdg/", env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn parses_npm_packument_metadata() {
        let metadata = parse_npm_metadata(
            r#"{
                "dist-tags": { "latest": "1.2.3" },
                "version": "0.1.0",
                "license": "MIT",
                "repository": { "url": "https://github.com/example/pkg.git" },
                "description": "demo package",
                "homepage": "https://example.com/pkg"
            }"#,
        )
        .expect("metadata");

        assert_eq!(metadata.version.as_deref(), Some("1.2.3"));
        assert_eq!(metadata.license.as_deref(), Some("MIT"));
        assert_eq!(
            metadata.repository.as_deref(),
            Some("https://github.com/example/pkg.git")
        );
        assert_eq!(metadata.description.as_deref(), Some("demo package"));
        assert_eq!(
            metadata.homepage.as_deref(),
            Some("https://example.com/pkg")
        );
        assert_eq!(metadata.downloads, None);
    }

    #[test]
    fn parses_npm_packument_version_fallback() {
        let metadata = parse_npm_metadata(
            r#"{
                "dist-tags": {},
                "version": "0.1.0"
            }"#,
        )
        .expect("metadata");

        assert_eq!(metadata.version.as_deref(), Some("0.1.0"));
    }

    #[test]
    fn parses_crates_metadata() {
        let metadata = parse_crates_metadata(
            r#"{
                "crate": {
                    "max_version": "2.0.0",
                    "license": "Apache-2.0",
                    "repository": "https://github.com/example/crate",
                    "description": "demo crate",
                    "downloads": 42,
                    "homepage": "https://example.com/crate"
                }
            }"#,
        )
        .expect("metadata");

        assert_eq!(metadata.version.as_deref(), Some("2.0.0"));
        assert_eq!(metadata.license.as_deref(), Some("Apache-2.0"));
        assert_eq!(
            metadata.repository.as_deref(),
            Some("https://github.com/example/crate")
        );
        assert_eq!(metadata.description.as_deref(), Some("demo crate"));
        assert_eq!(metadata.downloads, Some(42));
        assert_eq!(
            metadata.homepage.as_deref(),
            Some("https://example.com/crate")
        );
    }
}
