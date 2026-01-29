use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ecosystem {
    Node,
    MoonBit,
    Rust,
}

#[derive(Debug, Clone)]
pub struct ManifestPaths {
    pub package_json: Option<PathBuf>,
    pub moon_mod: Option<PathBuf>,
    pub cargo_toml: Option<PathBuf>,
    pub package_json_all: Vec<PathBuf>,
    pub moon_mod_all: Vec<PathBuf>,
    pub cargo_toml_all: Vec<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct ProjectContext {
    pub root: PathBuf,
    pub ecosystem: Option<Ecosystem>,
    pub manifests: ManifestPaths,
    pub git: Option<GitContext>,
}

impl ProjectContext {
    pub fn has_moonbit(&self) -> bool {
        self.manifests.moon_mod.is_some()
    }
}

pub fn detect_project_root(current_dir: &Path) -> anyhow::Result<PathBuf> {
    if let Ok(root) = git_root(current_dir) {
        return Ok(root);
    }
    Ok(current_dir.to_path_buf())
}

pub fn detect_manifests(
    root: &Path,
    current_dir: &Path,
    max_depth: usize,
) -> anyhow::Result<ManifestPaths> {
    let mut manifests = ManifestPaths {
        package_json: None,
        moon_mod: None,
        cargo_toml: None,
        package_json_all: Vec::new(),
        moon_mod_all: Vec::new(),
        cargo_toml_all: Vec::new(),
    };

    let candidates = [
        root.join("package.json"),
        root.join("moon.mod.json"),
        root.join("Cargo.toml"),
    ];

    for candidate in candidates {
        if candidate.exists() {
            match candidate.file_name().and_then(|n| n.to_str()) {
                Some("package.json") => manifests.package_json_all.push(candidate),
                Some("moon.mod.json") => manifests.moon_mod_all.push(candidate),
                Some("Cargo.toml") => manifests.cargo_toml_all.push(candidate),
                _ => {}
            }
        }
    }

    let walker = walkdir::WalkDir::new(root)
        .follow_links(false)
        .max_depth(max_depth)
        .into_iter()
        .filter_entry(|entry| !is_ignored(entry, root));
    for entry in walker.flatten() {
        if !entry.file_type().is_file() {
            continue;
        }
        let file_name = entry.file_name().to_string_lossy();
        match file_name.as_ref() {
            "package.json" => manifests.package_json_all.push(entry.path().to_path_buf()),
            "moon.mod.json" => manifests.moon_mod_all.push(entry.path().to_path_buf()),
            "Cargo.toml" => manifests.cargo_toml_all.push(entry.path().to_path_buf()),
            _ => {}
        }
    }

    manifests.package_json = choose_closest(current_dir, &manifests.package_json_all);
    manifests.moon_mod = choose_closest(current_dir, &manifests.moon_mod_all);
    manifests.cargo_toml = choose_closest(current_dir, &manifests.cargo_toml_all);

    Ok(manifests)
}

pub fn detect_ecosystem(manifests: &ManifestPaths) -> Option<Ecosystem> {
    if manifests.package_json.is_some() {
        return Some(Ecosystem::Node);
    }
    if manifests.moon_mod.is_some() {
        return Some(Ecosystem::MoonBit);
    }
    if manifests.cargo_toml.is_some() {
        return Some(Ecosystem::Rust);
    }
    None
}

pub fn build_context(current_dir: &Path) -> anyhow::Result<ProjectContext> {
    let root = detect_project_root(current_dir)?;
    let manifests = detect_manifests(&root, current_dir, 3)?;
    let ecosystem = detect_ecosystem(&manifests);
    let git = git_context(&root).ok();
    Ok(ProjectContext {
        root,
        ecosystem,
        manifests,
        git,
    })
}

fn git_root(current_dir: &Path) -> anyhow::Result<PathBuf> {
    let output = std::process::Command::new("git")
        .arg("rev-parse")
        .arg("--show-toplevel")
        .current_dir(current_dir)
        .output()?;
    if !output.status.success() {
        anyhow::bail!("git root not found");
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        anyhow::bail!("git root empty");
    }
    Ok(PathBuf::from(text))
}

#[derive(Debug, Clone)]
pub struct GitContext {
    pub root: PathBuf,
    pub remote: Option<String>,
    pub owner: Option<String>,
    pub repo: Option<String>,
    pub default_branch: Option<String>,
}

fn git_context(root: &Path) -> anyhow::Result<GitContext> {
    let remote = git_remote_url(root).ok();
    let (owner, repo) = infer_owner_repo(&remote);
    let default_branch = git_default_branch(root).ok();
    Ok(GitContext {
        root: root.to_path_buf(),
        remote,
        owner,
        repo,
        default_branch,
    })
}

fn git_remote_url(root: &Path) -> anyhow::Result<String> {
    let output = std::process::Command::new("git")
        .arg("remote")
        .arg("get-url")
        .arg("origin")
        .current_dir(root)
        .output()?;
    if !output.status.success() {
        anyhow::bail!("git remote missing");
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        anyhow::bail!("git remote empty");
    }
    Ok(text)
}

fn git_default_branch(root: &Path) -> anyhow::Result<String> {
    let output = std::process::Command::new("git")
        .arg("symbolic-ref")
        .arg("refs/remotes/origin/HEAD")
        .current_dir(root)
        .output()?;
    if !output.status.success() {
        anyhow::bail!("default branch not found");
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let branch = text.split('/').last().unwrap_or("").to_string();
    if branch.is_empty() {
        anyhow::bail!("default branch empty");
    }
    Ok(branch)
}

fn infer_owner_repo(remote: &Option<String>) -> (Option<String>, Option<String>) {
    let url = match remote {
        Some(url) => url,
        None => return (None, None),
    };
    let cleaned = url
        .trim()
        .trim_end_matches(".git")
        .replace("git+", "")
        .replace("git://", "https://")
        .replace(':', "/");
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

fn is_ignored(entry: &walkdir::DirEntry, root: &Path) -> bool {
    let rel = entry.path().strip_prefix(root).unwrap_or(entry.path());
    let mut components = rel.components();
    let first = match components.next() {
        Some(component) => component.as_os_str().to_string_lossy(),
        None => return false,
    };
    if matches!(
        first.as_ref(),
        ".git" | "target" | "node_modules" | "dist" | "build" | "out" | "vendor"
    ) {
        return true;
    }
    if first == "tests" {
        let rel_str = rel.to_string_lossy();
        if rel_str.starts_with("tests/fixtures") {
            return true;
        }
    }
    false
}

fn choose_closest(current_dir: &Path, paths: &[PathBuf]) -> Option<PathBuf> {
    let mut best: Option<(usize, String, PathBuf)> = None;
    for path in paths {
        let distance = path_distance(current_dir, path);
        let key = path.to_string_lossy().to_string();
        match &best {
            Some((best_distance, best_key, _)) => {
                if distance < *best_distance || (distance == *best_distance && key < *best_key) {
                    best = Some((distance, key, path.clone()));
                }
            }
            None => best = Some((distance, key, path.clone())),
        }
    }
    best.map(|(_, _, path)| path)
}

fn path_distance(from: &Path, to_file: &Path) -> usize {
    let to_dir = to_file.parent().unwrap_or_else(|| Path::new(""));
    let from_components: Vec<_> = from.components().collect();
    let to_components: Vec<_> = to_dir.components().collect();
    let common = from_components
        .iter()
        .zip(to_components.iter())
        .take_while(|(a, b)| a == b)
        .count();
    (from_components.len() - common) + (to_components.len() - common)
}
