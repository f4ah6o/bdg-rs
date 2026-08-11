use crate::readme::write_readme_atomic;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ReadmePlan {
    path: PathBuf,
    original: String,
    updated: String,
}

impl ReadmePlan {
    pub fn new(path: impl Into<PathBuf>, original: String, updated: String) -> Self {
        Self {
            path: path.into(),
            original,
            updated,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn changed(&self) -> bool {
        self.original != self.updated
    }

    pub fn diff(&self) -> String {
        if !self.changed() {
            return String::new();
        }
        let rel_path = self
            .path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("README.md");
        let patch = diffy::create_patch(&self.original, &self.updated);
        diffy::PatchFormatter::new()
            .fmt_patch(&patch)
            .to_string()
            .replace("--- original\n", &format!("--- a/{rel_path}\n"))
            .replace("+++ modified\n", &format!("+++ b/{rel_path}\n"))
    }

    pub fn apply(&self) -> anyhow::Result<()> {
        if self.changed() {
            write_readme_atomic(&self.path, &self.updated)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::ReadmePlan;
    use std::path::PathBuf;

    #[test]
    fn unchanged_plan_has_no_diff() {
        let plan = ReadmePlan::new(PathBuf::from("README.md"), "a\n".into(), "a\n".into());
        assert!(!plan.changed());
        assert!(plan.diff().is_empty());
    }

    #[test]
    fn changed_plan_has_stable_readme_paths() {
        let plan = ReadmePlan::new(PathBuf::from("docs/README.md"), "a\n".into(), "b\n".into());
        let diff = plan.diff();
        assert!(diff.contains("--- a/README.md"));
        assert!(diff.contains("+++ b/README.md"));
    }
}
