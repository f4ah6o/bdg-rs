use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BadgeKind {
    Version,
    Ci,
    License,
    Release,
    Docs,
    Downloads,
    Coverage,
    Msrv,
    Stars,
    Forks,
    Issues,
    PullRequests,
    Activity,
}

impl BadgeKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Version => "version",
            Self::Ci => "ci",
            Self::License => "license",
            Self::Release => "release",
            Self::Docs => "docs",
            Self::Downloads => "downloads",
            Self::Coverage => "coverage",
            Self::Msrv => "msrv",
            Self::Stars => "stars",
            Self::Forks => "forks",
            Self::Issues => "issues",
            Self::PullRequests => "pulls",
            Self::Activity => "activity",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Badge {
    pub kind: BadgeKind,
    pub label: String,
    pub image_url: String,
    pub link_url: Option<String>,
    /// Whether `bdg sync` should include this badge without an explicit `--only`.
    /// Optional/community badges remain available through `bdg add` and `--only`.
    pub sync_default: bool,
}

impl Badge {
    pub fn render_markdown(&self) -> String {
        match &self.link_url {
            Some(link) => format!("[![{}]({})]({})", self.label, self.image_url, link),
            None => format!("![{}]({})", self.label, self.image_url),
        }
    }
}

pub fn dedupe_badges(badges: Vec<Badge>) -> Vec<Badge> {
    let mut seen = HashSet::new();
    badges
        .into_iter()
        .filter(|badge| seen.insert((badge.kind, badge.image_url.clone(), badge.link_url.clone())))
        .collect()
}

fn badge(
    kind: BadgeKind,
    label: impl Into<String>,
    image_url: impl Into<String>,
    link_url: Option<String>,
) -> Badge {
    Badge {
        kind,
        label: label.into(),
        image_url: image_url.into(),
        link_url,
        sync_default: true,
    }
}

fn optional_badge(
    kind: BadgeKind,
    label: impl Into<String>,
    image_url: impl Into<String>,
    link_url: Option<String>,
) -> Badge {
    Badge {
        sync_default: false,
        ..badge(kind, label, image_url, link_url)
    }
}

pub fn badge_for_npm(package: &str) -> Badge {
    badge(
        BadgeKind::Version,
        "npm",
        format!("https://img.shields.io/npm/v/{}.svg", package),
        Some(format!("https://www.npmjs.com/package/{}", package)),
    )
}

pub fn badge_for_crates(crate_name: &str) -> Badge {
    badge(
        BadgeKind::Version,
        "crates.io",
        format!("https://img.shields.io/crates/v/{}.svg", crate_name),
        Some(format!("https://crates.io/crates/{}", crate_name)),
    )
}

pub fn badge_for_npm_downloads(package: &str) -> Badge {
    badge(
        BadgeKind::Downloads,
        "npm downloads",
        format!("https://img.shields.io/npm/dt/{}.svg", package),
        Some(format!("https://www.npmjs.com/package/{}", package)),
    )
}

pub fn badge_for_crates_downloads(crate_name: &str) -> Badge {
    badge(
        BadgeKind::Downloads,
        "crates.io downloads",
        format!("https://img.shields.io/crates/d/{}.svg", crate_name),
        Some(format!("https://crates.io/crates/{}", crate_name)),
    )
}

pub fn badge_for_crates_msrv(crate_name: &str) -> Badge {
    optional_badge(
        BadgeKind::Msrv,
        "MSRV",
        format!("https://img.shields.io/crates/msrv/{}.svg", crate_name),
        Some(format!("https://crates.io/crates/{}", crate_name)),
    )
}

pub fn badge_for_docs_rs(crate_name: &str) -> Badge {
    badge(
        BadgeKind::Docs,
        "docs.rs",
        format!("https://docs.rs/{}/badge.svg", crate_name),
        Some(format!("https://docs.rs/{}", crate_name)),
    )
}

pub fn badge_for_docs_url(url: &str) -> Badge {
    badge(
        BadgeKind::Docs,
        "docs",
        "https://img.shields.io/badge/docs-online-blue.svg",
        Some(url.to_string()),
    )
}

pub fn badge_for_moonbit(module: &str) -> Badge {
    let link_url = if module.contains('/') {
        Some(format!("https://mooncakes.io/docs/{}", module))
    } else {
        Some("https://mooncakes.io/".to_string())
    };

    badge(
        BadgeKind::Version,
        "moonbit",
        format!(
            "https://img.shields.io/badge/moonbit-{}-informational",
            module
        ),
        link_url,
    )
}

pub fn badge_for_license(owner: &str, repo: &str) -> Badge {
    badge(
        BadgeKind::License,
        "license",
        format!(
            "https://img.shields.io/github/license/{}/{}.svg",
            owner, repo
        ),
        Some(format!("https://github.com/{}/{}", owner, repo)),
    )
}

pub fn badge_for_license_text(license: &str, repository: Option<&str>) -> Badge {
    let link_url = if is_dual_license_expression(license) {
        None
    } else {
        repository.map(str::to_string)
    };

    badge(
        BadgeKind::License,
        "license",
        format!(
            "https://img.shields.io/badge/license-{}-blue.svg",
            encode_static_badge_segment(license)
        ),
        link_url,
    )
}

pub fn badge_for_github_release(owner: &str, repo: &str) -> Badge {
    badge(
        BadgeKind::Release,
        "release",
        format!(
            "https://img.shields.io/github/v/release/{}/{}.svg",
            owner, repo
        ),
        Some(format!("https://github.com/{}/{}/releases", owner, repo)),
    )
}

pub fn badge_for_github_downloads(owner: &str, repo: &str) -> Badge {
    optional_badge(
        BadgeKind::Downloads,
        "GitHub downloads",
        format!(
            "https://img.shields.io/github/downloads/{}/{}/total.svg",
            owner, repo
        ),
        Some(format!("https://github.com/{}/{}/releases", owner, repo)),
    )
}

pub fn badge_for_github_stars(owner: &str, repo: &str) -> Badge {
    optional_badge(
        BadgeKind::Stars,
        "GitHub stars",
        format!("https://img.shields.io/github/stars/{}/{}.svg", owner, repo),
        Some(format!("https://github.com/{}/{}/stargazers", owner, repo)),
    )
}

pub fn badge_for_github_forks(owner: &str, repo: &str) -> Badge {
    optional_badge(
        BadgeKind::Forks,
        "GitHub forks",
        format!("https://img.shields.io/github/forks/{}/{}.svg", owner, repo),
        Some(format!("https://github.com/{}/{}/forks", owner, repo)),
    )
}

pub fn badge_for_github_issues(owner: &str, repo: &str) -> Badge {
    optional_badge(
        BadgeKind::Issues,
        "GitHub issues",
        format!(
            "https://img.shields.io/github/issues/{}/{}.svg",
            owner, repo
        ),
        Some(format!("https://github.com/{}/{}/issues", owner, repo)),
    )
}

pub fn badge_for_github_pull_requests(owner: &str, repo: &str) -> Badge {
    optional_badge(
        BadgeKind::PullRequests,
        "GitHub pull requests",
        format!(
            "https://img.shields.io/github/issues-pr/{}/{}.svg",
            owner, repo
        ),
        Some(format!("https://github.com/{}/{}/pulls", owner, repo)),
    )
}

pub fn badge_for_github_last_commit(owner: &str, repo: &str) -> Badge {
    optional_badge(
        BadgeKind::Activity,
        "GitHub last commit",
        format!(
            "https://img.shields.io/github/last-commit/{}/{}.svg",
            owner, repo
        ),
        Some(format!("https://github.com/{}/{}/commits", owner, repo)),
    )
}

pub fn badge_for_codecov(owner: &str, repo: &str) -> Badge {
    badge(
        BadgeKind::Coverage,
        "codecov",
        format!(
            "https://img.shields.io/codecov/c/github/{}/{}.svg",
            owner, repo
        ),
        Some(format!("https://codecov.io/gh/{}/{}", owner, repo)),
    )
}

pub fn badge_for_workflow(owner: &str, repo: &str, workflow_file: &str) -> Badge {
    badge(
        BadgeKind::Ci,
        "CI",
        format!(
            "https://github.com/{}/{}/actions/workflows/{}/badge.svg",
            owner, repo, workflow_file
        ),
        Some(format!(
            "https://github.com/{}/{}/actions/workflows/{}",
            owner, repo, workflow_file
        )),
    )
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

fn is_dual_license_expression(license: &str) -> bool {
    license.contains(" OR ") || license.contains(" AND ") || license.contains('/')
}

#[cfg(test)]
mod tests {
    use super::{
        badge_for_codecov, badge_for_crates_downloads, badge_for_crates_msrv, badge_for_docs_rs,
        badge_for_docs_url, badge_for_github_downloads, badge_for_github_forks,
        badge_for_github_issues, badge_for_github_last_commit, badge_for_github_pull_requests,
        badge_for_github_release, badge_for_github_stars, badge_for_license_text,
        badge_for_npm_downloads, encode_static_badge_segment,
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
        assert_eq!(badge.link_url.as_deref(), None);
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

    #[test]
    fn optional_github_and_msrv_badges_are_supported_without_expanding_default_sync() {
        let badges = [
            badge_for_crates_msrv("bdg"),
            badge_for_github_downloads("f4ah6o", "bdg-rs"),
            badge_for_github_stars("f4ah6o", "bdg-rs"),
            badge_for_github_forks("f4ah6o", "bdg-rs"),
            badge_for_github_issues("f4ah6o", "bdg-rs"),
            badge_for_github_pull_requests("f4ah6o", "bdg-rs"),
            badge_for_github_last_commit("f4ah6o", "bdg-rs"),
        ];

        assert!(badges.iter().all(|badge| !badge.sync_default));
        assert!(badges[0].image_url.contains("/crates/msrv/bdg.svg"));
        assert!(
            badges[1]
                .image_url
                .contains("/github/downloads/f4ah6o/bdg-rs/total.svg")
        );
        assert!(
            badges[2]
                .image_url
                .contains("/github/stars/f4ah6o/bdg-rs.svg")
        );
        assert!(
            badges[3]
                .image_url
                .contains("/github/forks/f4ah6o/bdg-rs.svg")
        );
        assert!(
            badges[4]
                .image_url
                .contains("/github/issues/f4ah6o/bdg-rs.svg")
        );
        assert!(
            badges[5]
                .image_url
                .contains("/github/issues-pr/f4ah6o/bdg-rs.svg")
        );
        assert!(
            badges[6]
                .image_url
                .contains("/github/last-commit/f4ah6o/bdg-rs.svg")
        );
    }
}
