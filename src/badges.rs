#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BadgeKind {
    Version,
    Ci,
    License,
    Release,
    Docs,
    Downloads,
}

#[derive(Debug, Clone)]
pub struct Badge {
    pub kind: BadgeKind,
    pub label: String,
    pub image_url: String,
    pub link_url: Option<String>,
}

impl Badge {
    pub fn render_markdown(&self) -> String {
        match &self.link_url {
            Some(link) => format!("[![{}]({})]({})", self.label, self.image_url, link),
            None => format!("![{}]({})", self.label, self.image_url),
        }
    }
}

pub fn badge_for_npm(package: &str) -> Badge {
    Badge {
        kind: BadgeKind::Version,
        label: "npm".to_string(),
        image_url: format!("https://img.shields.io/npm/v/{}.svg", package),
        link_url: Some(format!("https://www.npmjs.com/package/{}", package)),
    }
}

pub fn badge_for_crates(crate_name: &str) -> Badge {
    Badge {
        kind: BadgeKind::Version,
        label: "crates.io".to_string(),
        image_url: format!("https://img.shields.io/crates/v/{}.svg", crate_name),
        link_url: Some(format!("https://crates.io/crates/{}", crate_name)),
    }
}

pub fn badge_for_moonbit(module: &str) -> Badge {
    let link_url = if module.contains('/') {
        // Format: username/package -> https://mooncakes.io/docs/username/package
        Some(format!("https://mooncakes.io/docs/{}", module))
    } else {
        // Fallback to mooncakes.io homepage for unpublished modules
        Some("https://mooncakes.io/".to_string())
    };

    Badge {
        kind: BadgeKind::Version,
        label: "moonbit".to_string(),
        image_url: format!(
            "https://img.shields.io/badge/moonbit-{}-informational",
            module
        ),
        link_url,
    }
}

pub fn badge_for_license(owner: &str, repo: &str) -> Badge {
    Badge {
        kind: BadgeKind::License,
        label: "license".to_string(),
        image_url: format!(
            "https://img.shields.io/github/license/{}/{}.svg",
            owner, repo
        ),
        link_url: Some(format!("https://github.com/{}/{}", owner, repo)),
    }
}

pub fn badge_for_workflow(owner: &str, repo: &str, workflow: &str) -> Badge {
    Badge {
        kind: BadgeKind::Ci,
        label: "CI".to_string(),
        image_url: format!(
            "https://github.com/{}/{}/actions/workflows/{}.yaml/badge.svg",
            owner, repo, workflow
        ),
        link_url: Some(format!(
            "https://github.com/{}/{}/actions/workflows/{}.yaml",
            owner, repo, workflow
        )),
    }
}
