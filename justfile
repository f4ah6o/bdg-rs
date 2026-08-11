# Local development and release tasks

check:
    cargo fmt --check
    cargo clippy --all-targets --all-features -- -D warnings
    cargo test --all-targets

release-check: check
    cargo build --release --all-features
    cargo publish --dry-run

release: release-check
    version=$(grep '^version = ' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/'); \
    git tag "v${version}"; \
    git push origin "v${version}"
