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
        if ext != "yaml" && ext != "yaml" {
            continue;
        }
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            workflows.push(WorkflowInfo {
                name: stem.to_string(),
                file: format!("{}.{}", stem, ext),
            });
        }
    }
    workflows
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
    let gh_check = std::process::Command::new("gh").arg("--version").output();
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
            }
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
