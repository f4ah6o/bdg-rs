use std::path::Path;

#[derive(Debug, Clone)]
pub struct WorkflowInfo {
    pub name: String,
    pub file: String,
}

pub fn detect_workflows(root: &Path) -> Vec<WorkflowInfo> {
    let workflows_dir = root.join(".github").join("workflows");
    let mut workflows = Vec::new();
    if !workflows_dir.exists() {
        return workflows;
    }
    let entries = match std::fs::read_dir(workflows_dir) {
        Ok(entries) => entries,
        Err(_) => return workflows,
    };
    for entry in entries.flatten() {
        if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
            continue;
        }
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "yaml" && ext != "yml" {
            continue;
        }
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            workflows.push(WorkflowInfo {
                name: stem.to_string(),
                file: format!("{}.{}", stem, ext),
            });
        }
    }
    workflows.sort_by(|a, b| a.file.cmp(&b.file));
    workflows
}

pub fn detects_codecov(root: &Path) -> bool {
    if [
        ".codecov.yml",
        ".codecov.yaml",
        "codecov.yml",
        "codecov.yaml",
    ]
    .iter()
    .any(|candidate| root.join(candidate).is_file())
    {
        return true;
    }

    let workflows_dir = root.join(".github").join("workflows");
    let Ok(entries) = std::fs::read_dir(workflows_dir) else {
        return false;
    };
    entries.flatten().any(|entry| {
        let path = entry.path();
        let ext = path.extension().and_then(|value| value.to_str());
        if !matches!(ext, Some("yml" | "yaml")) {
            return false;
        }
        std::fs::read_to_string(path)
            .map(|content| content.to_ascii_lowercase().contains("codecov"))
            .unwrap_or(false)
    })
}

pub fn gh_latest_status(workflow: &str) -> Option<(String, String)> {
    let view_output = std::process::Command::new("gh")
        .arg("workflow")
        .arg("view")
        .arg(workflow)
        .output()
        .ok()?;
    if !view_output.status.success() {
        return None;
    }
    let run_output = std::process::Command::new("gh")
        .arg("run")
        .arg("list")
        .arg("--limit")
        .arg("1")
        .arg("--workflow")
        .arg(workflow)
        .output()
        .ok()?;
    if !run_output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&run_output.stdout);
    let mut parts = text.lines().next().unwrap_or("").split_whitespace();
    let status = parts.next().unwrap_or("").to_string();
    let timestamp = parts.next().unwrap_or("").to_string();
    if status.is_empty() {
        None
    } else {
        Some((status, timestamp))
    }
}

#[derive(Debug, Clone)]
pub struct GhRunInfo {
    pub ok: bool,
    pub reason: Option<String>,
    pub conclusion: Option<String>,
    pub run_id: Option<u64>,
    pub html_url: Option<String>,
    pub updated_at: Option<String>,
}

pub fn gh_latest_status_json(workflow: &str) -> GhRunInfo {
    let current_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    gh_latest_status_json_in(&current_dir, workflow)
}

pub fn gh_latest_status_json_in(root: &Path, workflow: &str) -> GhRunInfo {
    let gh_check = std::process::Command::new("gh")
        .arg("--version")
        .current_dir(root)
        .output();
    if gh_check.is_err() {
        return GhRunInfo {
            ok: false,
            reason: Some("gh_unavailable".to_string()),
            conclusion: None,
            run_id: None,
            html_url: None,
            updated_at: None,
        };
    }
    let output = std::process::Command::new("gh")
        .arg("run")
        .arg("list")
        .arg("--workflow")
        .arg(workflow)
        .arg("--limit")
        .arg("1")
        .arg("--json")
        .arg("conclusion,updatedAt,url,databaseId")
        .current_dir(root)
        .output();
    let output = match output {
        Ok(output) => output,
        Err(_) => {
            return GhRunInfo {
                ok: false,
                reason: Some("gh_unavailable".to_string()),
                conclusion: None,
                run_id: None,
                html_url: None,
                updated_at: None,
            };
        }
    };
    if !output.status.success() {
        return GhRunInfo {
            ok: false,
            reason: Some("auth_required".to_string()),
            conclusion: None,
            run_id: None,
            html_url: None,
            updated_at: None,
        };
    }
    let text = String::from_utf8_lossy(&output.stdout).to_string();
    let runs: Vec<GhRunPayload> = serde_json::from_str(&text).unwrap_or_default();
    if runs.is_empty() {
        return GhRunInfo {
            ok: false,
            reason: Some("no_runs".to_string()),
            conclusion: None,
            run_id: None,
            html_url: None,
            updated_at: None,
        };
    }
    let run = &runs[0];
    GhRunInfo {
        ok: true,
        reason: None,
        conclusion: run.conclusion.clone(),
        run_id: run.database_id,
        html_url: run.url.clone(),
        updated_at: run.updated_at.clone(),
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
struct GhRunPayload {
    conclusion: Option<String>,
    #[serde(rename = "databaseId")]
    database_id: Option<u64>,
    #[serde(rename = "updatedAt")]
    updated_at: Option<String>,
    url: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::detects_codecov;

    #[test]
    fn detects_codecov_from_config_or_workflow() {
        let temp = tempfile::tempdir().unwrap();
        assert!(!detects_codecov(temp.path()));

        std::fs::write(temp.path().join(".codecov.yml"), "coverage: {}\n").unwrap();
        assert!(detects_codecov(temp.path()));
        std::fs::remove_file(temp.path().join(".codecov.yml")).unwrap();

        let workflows = temp.path().join(".github/workflows");
        std::fs::create_dir_all(&workflows).unwrap();
        std::fs::write(
            workflows.join("ci.yml"),
            "steps:\n  - uses: codecov/codecov-action@v5\n",
        )
        .unwrap();
        assert!(detects_codecov(temp.path()));
    }
}
