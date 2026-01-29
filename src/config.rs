use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub version: VersionConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VersionConfig {
    #[serde(default = "default_allow_yy")]
    pub allow_yy_calver: bool,
    #[serde(default = "default_year_min")]
    pub year_min: i32,
    #[serde(default = "default_year_max")]
    pub year_max: i32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: VersionConfig::default(),
        }
    }
}

impl Default for VersionConfig {
    fn default() -> Self {
        Self {
            allow_yy_calver: default_allow_yy(),
            year_min: default_year_min(),
            year_max: default_year_max(),
        }
    }
}

pub fn load_config(current_dir: &Path, git_root: &Path) -> anyhow::Result<Config> {
    let mut dir = current_dir.to_path_buf();
    loop {
        let candidate = dir.join(".bdg.toml");
        if candidate.exists() {
            return read_config(&candidate);
        }
        if dir == git_root {
            break;
        }
        if !dir.pop() {
            break;
        }
    }
    Ok(Config::default())
}

fn read_config(path: &Path) -> anyhow::Result<Config> {
    let content = std::fs::read_to_string(path)?;
    let config: Config = toml::from_str(&content)?;
    Ok(config)
}

fn default_allow_yy() -> bool {
    false
}

fn default_year_min() -> i32 {
    2000
}

fn default_year_max() -> i32 {
    2199
}
