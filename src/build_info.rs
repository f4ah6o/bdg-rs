pub const RELEASE_COMMIT: &str = include_str!("release-commit.txt");

pub fn version_string() -> String {
    format!(
        "{} {} ({})",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        RELEASE_COMMIT
    )
}

#[cfg(test)]
mod tests {
    use super::{RELEASE_COMMIT, version_string};

    #[test]
    fn version_includes_release_source() {
        assert!(!RELEASE_COMMIT.is_empty());
        assert_eq!(
            version_string(),
            format!(
                "{} {} ({})",
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION"),
                RELEASE_COMMIT
            )
        );
    }
}
