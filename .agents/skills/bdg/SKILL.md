---
name: bdg
description: Use bdg to inspect and manage README badge blocks safely in Rust, Node, and MoonBit repositories.
---

# bdg

Use `bdg` when you need to inspect, add, or remove badges inside a README without touching unrelated content.

## When To Use

- You need to add badges for crates.io, npm, MoonBit, license, or GitHub Actions.
- You need to inspect the current managed badge block before changing it.
- You need to remove badges by id, kind, or all at once while keeping edits constrained.

## Command Guide

### `bdg add`

Adds badges to the managed block in the detected README.

- `bdg add`
- `bdg add --yes`
- `bdg add --only ci,version,license`
- `bdg add --dry-run`
- `bdg add --json --dry-run`

Behavior:

- Detects project metadata from `Cargo.toml`, `package.json`, or `moon.mod.json`.
- Detects GitHub Actions workflows from `.github/workflows`.
- Writes only inside the `<!-- bdg:begin -->` / `<!-- bdg:end -->` block.
- Inserts the marker block if it does not exist yet.

### `bdg list`

Shows the current README badge state and detected project context.

- `bdg list`
- `bdg list --json`
- `bdg list --quiet`

Behavior:

- Reports README path, marker status, badge count, and CI workflow status.
- `--json` emits structured repository, manifest, registry, CI, and managed-block data.

### `bdg remove`

Removes badges from the managed block.

- `bdg remove`
- `bdg remove --id ci:rust.yaml`
- `bdg remove --kind github_actions`
- `bdg remove --all`
- `bdg remove --dry-run`
- `bdg remove --json --dry-run`

Behavior:

- Interactive mode lets you choose which badges to remove.
- `--id` and `--kind` target removals precisely.
- `--all` removes the full managed block content.
- `--dry-run` shows the diff without writing.

### `bdg skills`

Prints this Agent Skills document to stdout.

Use it when another agent or tool needs the current `bdg` usage context in a standards-aligned format.

## Constraints

- `bdg` only manages content inside its marker block.
- `bdg` does not rewrite unrelated README sections.
- `bdg add` and `bdg remove` may modify files; prefer `--dry-run` first when changes need review.
- `bdg list` and `bdg skills` are read-only.

## Project Detection

- Rust: `Cargo.toml`
- Node: `package.json`
- MoonBit: `moon.mod.json`

`bdg` chooses the closest matching manifest from the current directory within the repository and searches config from the current directory up to the git root.
