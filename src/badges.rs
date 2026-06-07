#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BadgeKind {
    Version,
    Ci,
    License,
    Release,
    Docs,
    Downloads,
    Coverage,
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

pub fn badge_for_npm_downloads(package: &str) -> Badge {
    Badge {
        kind: BadgeKind::Downloads,
        label: "npm downloads".to_string(),
        image_url: format!("https://img.shields.io/npm/dt/{}.svg", package),
        link_url: Some(format!("https://www.npmjs.com/package/{}", package)),
    }
}

pub fn badge_for_crates_downloads(crate_name: &str) -> Badge {
    Badge {
        kind: BadgeKind::Downloads,
        label: "crates.io downloads".to_string(),
        image_url: format!("https://img.shields.io/crates/d/{}.svg", crate_name),
        link_url: Some(format!("https://crates.io/crates/{}", crate_name)),
    }
}

pub fn badge_for_docs_rs(crate_name: &str) -> Badge {
    Badge {
        kind: BadgeKind::Docs,
        label: "docs.rs".to_string(),
        image_url: format!("https://docs.rs/{}/badge.svg", crate_name),
        link_url: Some(format!("https://docs.rs/{}", crate_name)),
    }
}

pub fn badge_for_docs_url(url: &str) -> Badge {
    Badge {
        kind: BadgeKind::Docs,
        label: "docs".to_string(),
        image_url: "https://img.shields.io/badge/docs-online-blue.svg".to_string(),
        link_url: Some(url.to_string()),
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

pub fn badge_for_license_text(license: &str, repository: Option<&str>) -> Badge {
    Badge {
        kind: BadgeKind::License,
        label: "license".to_string(),
        image_url: format!(
            "https://img.shields.io/badge/license-{}-blue.svg",
            encode_static_badge_segment(license)
        ),
        link_url: repository.map(str::to_string),
    }
}

pub fn badge_for_github_release(owner: &str, repo: &str) -> Badge {
    Badge {
        kind: BadgeKind::Release,
        label: "release".to_string(),
        image_url: format!(
            "https://img.shields.io/github/v/release/{}/{}.svg",
            owner, repo
        ),
        link_url: Some(format!("https://github.com/{}/{}/releases", owner, repo)),
    }
}

pub fn badge_for_codecov(owner: &str, repo: &str) -> Badge {
    Badge {
        kind: BadgeKind::Coverage,
        label: "codecov".to_string(),
        image_url: format!(
            "https://img.shields.io/codecov/c/github/{}/{}.svg",
            owner, repo
        ),
        link_url: Some(format!("https://codecov.io/gh/{}/{}", owner, repo)),
    }
}

pub fn badge_for_workflow(owner: &str, repo: &str, workflow_file: &str) -> Badge {
    Badge {
        kind: BadgeKind::Ci,
        label: "CI".to_string(),
        image_url: format!(
            "https://github.com/{}/{}/actions/workflows/{}/badge.svg",
            owner, repo, workflow_file
        ),
        link_url: Some(format!(
            "https://github.com/{}/{}/actions/workflows/{}",
            owner, repo, workflow_file
        )),
    }
}

fn encode_static_badge_segment(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_' | b'.' => encoded.push(byte as char),
            b'-' => encoded.push_str("--"),
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::{
        badge_for_codecov, badge_for_crates_downloads, badge_for_docs_rs, badge_for_docs_url,
        badge_for_github_release, badge_for_license_text, badge_for_npm_downloads,
        encode_static_badge_segment,
    };

    #[test]
    fn static_badge_segment_escapes_shields_separator() {
        assert_eq!(encode_static_badge_segment("MIT"), "MIT");
        assert_eq!(
            encode_static_badge_segment("MIT OR Apache-2.0"),
            "MIT%20OR%20Apache--2.0"
        );
    }

    #[test]
    fn license_text_badge_renders_static_shields_badge() {
        let badge = badge_for_license_text(
            "MIT OR Apache-2.0",
            Some("https://github.com/f4ah6o/shuttle-rs"),
        );

        assert_eq!(
            badge.image_url,
            "https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg"
        );
        assert_eq!(
            badge.link_url.as_deref(),
            Some("https://github.com/f4ah6o/shuttle-rs")
        );
    }

    #[test]
    fn practical_badges_render_expected_markdown() {
        assert_eq!(
            badge_for_npm_downloads("@scope/pkg").render_markdown(),
            "[![npm downloads](https://img.shields.io/npm/dt/@scope/pkg.svg)](https://www.npmjs.com/package/@scope/pkg)"
        );
        assert_eq!(
            badge_for_crates_downloads("bdg").render_markdown(),
            "[![crates.io downloads](https://img.shields.io/crates/d/bdg.svg)](https://crates.io/crates/bdg)"
        );
        assert_eq!(
            badge_for_docs_rs("bdg").render_markdown(),
            "[![docs.rs](https://docs.rs/bdg/badge.svg)](https://docs.rs/bdg)"
        );
        assert_eq!(
            badge_for_docs_url("https://example.com/docs").render_markdown(),
            "[![docs](https://img.shields.io/badge/docs-online-blue.svg)](https://example.com/docs)"
        );
        assert_eq!(
            badge_for_github_release("f4ah6o", "bdg-rs").render_markdown(),
            "[![release](https://img.shields.io/github/v/release/f4ah6o/bdg-rs.svg)](https://github.com/f4ah6o/bdg-rs/releases)"
        );
        assert_eq!(
            badge_for_codecov("f4ah6o", "bdg-rs").render_markdown(),
            "[![codecov](https://img.shields.io/codecov/c/github/f4ah6o/bdg-rs.svg)](https://codecov.io/gh/f4ah6o/bdg-rs)"
        );
    }
}
