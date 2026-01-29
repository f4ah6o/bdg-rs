# Release new version (tag + push)

release-check:
    cargo test --all --all-features
    cargo build --release --all-features
    cargo publish --dry-run

release: release-check
    version=$(grep '^version = ' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/'); \
    git tag "v${version}"; \
    git push origin "v${version}"
