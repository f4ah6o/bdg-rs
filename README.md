# bdg
<!-- bdg:begin -->
[![crates.io](https://img.shields.io/crates/v/bdg.svg)](https://crates.io/crates/bdg)
[![license](https://img.shields.io/github/license/f4ah6o/bdg-rs.svg)](https://github.com/f4ah6o/bdg-rs)
<!-- bdg:end -->

Interactive badge manager CLI for README files. It detects project metadata, suggests badges, and keeps edits confined to a managed marker block.

## Features
- Detects Node, Rust, and MoonBit manifests
- Reads registry metadata (npm, crates)
- Detects GitHub Actions workflows
- Adds/removes badges safely inside marker block
- Optional TUI with multi-select for add/remove

## Installation
```bash
cargo install bdg
```

## Usage
```bash
bdg add
bdg add --yes
bdg add --only ci,version,license
bdg add --dry-run

bdg list
bdg list --json

bdg remove
bdg remove --id ci:ci.yaml --id npm:@scope/pkg
bdg remove --kind github_actions
bdg remove --all
bdg remove --dry-run
```

## Managed Block
All edits are confined to the marker block:
```md
<!-- bdg:begin -->
[![crates.io](https://img.shields.io/crates/v/bdg.svg)](https://crates.io/crates/bdg)
<!-- bdg:end -->
```
If missing, bdg inserts it below the first H1 heading.

## TUI Keys
- Up/Down: move
- Space: toggle
- Enter: apply
- q/Esc/Ctrl+C: cancel

## Config (.bdg.toml)
bdg searches from current directory up to git root and uses the first config found.
```toml
[version]
allow_yy_calver = false
year_min = 2000
year_max = 2199
```

## Version Classification
Versions are classified as calver, semver, or unknown with calver priority.
Use `--allow-yy-calver` to opt in to YY.MM/YY.MM.MICRO calver patterns.

## Exit Codes
- 0: no change or success
- 2: changes detected in dry-run
- 1: error










