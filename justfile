# Local development and release tasks

check:
    cargo fmt --check
    cargo clippy --all-targets --all-features -- -D warnings
    cargo test --all-targets

release-check: check
    cargo build --release --all-features
    cargo publish --dry-run

release: release-check
    git tag -f latest HEAD
    git push -f origin latest
