# bdg
<!-- bdg:begin -->
[![crates.io](https://img.shields.io/crates/v/bdg.svg)](https://crates.io/crates/bdg)
[![CI](https://github.com/f4ah6o/bdg-rs/actions/workflows/ci.yaml/badge.svg)](https://github.com/f4ah6o/bdg-rs/actions/workflows/ci.yaml)
[![GitHub contributors](https://img.shields.io/github/contributors/f4ah6o/bdg-rs.svg)](https://github.com/f4ah6o/bdg-rs/graphs/contributors)
<!-- bdg:end -->

`bdg` is a focused CLI for discovering, validating, synchronizing, and removing README badges without rewriting unrelated documentation.

It treats badges as managed project metadata rather than ad-hoc Markdown: project manifests and CI configuration are discovered, a deterministic README change is planned, and only the `bdg` marker block is modified.

## Highlights

- Rust, Node, and MoonBit project detection
- crates.io and npm registry metadata
- GitHub Actions workflow discovery
- version, CI, license, release, docs, downloads, coverage, MSRV, and GitHub repository badges
- deterministic non-interactive `sync` for local automation and CI
- structural `check` with machine-readable JSON output
- dry-run diffs with exit code `2` when changes are pending
- targeted removal by stable badge id or kind
- declarative badge catalogs from built-in definitions, local files, or HTTP(S) URLs
- repository-aware config discovery
- `-C/--directory` for scripting without changing shell state
- optional TUI for interactive add/remove workflows
- README writes constrained to `<!-- bdg:begin -->` / `<!-- bdg:end -->`

## Installation

```bash
cargo install bdg
```

## Recommended workflow

Use `sync` when the README should reflect detected project metadata:

```bash
# Apply the canonical managed badge set.
bdg sync

# CI/pre-commit: do not write; exit 2 when synchronization is needed.
bdg sync --check

# Inspect the exact pending patch.
bdg sync --dry-run

# Restrict synchronization to selected badge classes.
bdg sync --only ci,version,license
```

Use `check` when you only want to validate the existing managed block. It performs no registry lookup and does not modify the README:

```bash
bdg check
bdg check --strict
bdg check --json
```

`--strict` treats unrecognized lines inside the managed block as errors. Without it, they are warnings.

## Supported badges

`bdg` distinguishes canonical project metadata from optional repository/community signals so broad badge support does not make an ordinary `bdg sync` noisy.

Canonical candidates can include:

- `version`: npm, crates.io, MoonBit
- `ci`: detected GitHub Actions workflows
- `license`: manifest license or GitHub repository license
- `release`: latest GitHub release
- `docs`: docs.rs or detected package documentation URL
- `downloads`: npm or crates.io downloads
- `coverage`: Codecov when configuration or workflow usage is detected

Additional supported candidates include:

- `msrv`: crates.io MSRV
- `downloads`: total GitHub release downloads
- `stars`: GitHub stars
- `forks`: GitHub forks
- `issues`: open GitHub issues
- `pulls`: open GitHub pull requests
- `activity`: GitHub last commit

These additional repository/community badges are available in interactive `bdg add`, in `bdg add --yes`, and through an explicit `sync --only`. They are intentionally not introduced by an unqualified `bdg sync`.

```bash
bdg sync --only msrv,stars,issues,activity
bdg add --only downloads,stars,forks,pulls
```

## Commands

### `bdg sync`

Reconciles the managed block non-interactively from detected project metadata.

```bash
bdg sync
bdg sync --only ci,version,license,release,docs,downloads,coverage
bdg sync --only msrv,stars,forks,issues,pulls,activity
bdg sync --check
bdg sync --dry-run
bdg sync --json --check
```

`--check` and `--dry-run` never write. They exit with code `2` when a change would be made.

### `bdg check`

Validates marker structure, managed badge syntax, and duplicate badge ids.

```bash
bdg check
bdg check --strict
bdg check --json
```

JSON output uses the `bdg.check/v1` schema.

### `bdg add`

Interactive/manual badge selection. `--yes` makes it non-interactive; for canonical automation prefer `bdg sync`.

```bash
bdg add
bdg add --yes
bdg add --only ci,version,license
bdg add --only msrv,stars,forks,issues,pulls,activity
bdg add --dry-run
```

### `bdg catalog`

Searches declarative badge definitions and adds them without recompiling `bdg`. The bundled catalog can be extended or overridden with project-local, file, or HTTP(S) sources.

```bash
bdg catalog search github
bdg catalog search size --json
bdg catalog add github-contributors
bdg catalog add github-repo-size --dry-run
bdg catalog add team-status --source ./team-catalog.toml
bdg catalog add remote-status --source https://example.com/bdg-catalog.json
bdg catalog add-url https://example.com/status.svg --label status --link https://example.com/status
```

Catalog sources may be TOML or JSON using schema `bdg.catalog/v1`. Definitions are templates and can use project placeholders: `{owner}`, `{repo}`, `{crate}`, `{package}`, `{module}`, and `{name}`. Missing values can be supplied explicitly with repeated `--set KEY=VALUE`.

```toml
schema = "bdg.catalog/v1"

[[badge]]
id = "website-status"
kind = "status"
label = "website"
image = "https://img.shields.io/website?url={site}"
link = "{site}"
requires = ["site"]
tags = ["website", "status"]
description = "Website availability"
```

```bash
bdg catalog add website-status --source ./catalog.toml --set site=https://example.com
```

For one-off badges, `catalog add-url` bypasses catalogs entirely. Arbitrary valid HTTP(S) image badges are treated as managed external badges, so custom services and Shields Endpoint badges remain compatible with `bdg check --strict`.

### `bdg list`

Inspects the current managed badges plus detected repository, manifest, registry, and CI metadata.

```bash
bdg list
bdg list --json
bdg list --quiet
```

`list` is read-only and reports the actual marker state; it does not synthesize a missing block.

### `bdg remove`

Removes managed badges interactively or by stable id/kind.

```bash
bdg remove
bdg remove --id ci:rust.yaml
bdg remove --id npm:@scope/pkg
bdg remove --kind github_actions
bdg remove --all
bdg remove --dry-run
bdg remove --json --dry-run
```

### `bdg skills`

Prints the bundled Agent Skills `SKILL.md` so agents can load the current CLI contract directly.

## Run against another directory

All commands support `-C/--directory`:

```bash
bdg -C ../project check
bdg --directory ../project sync --check
bdg -C ../project list --json
```

Relative paths are resolved from the process working directory. Config discovery still begins at the requested directory and stops at its Git root.

## Managed block

All writes are constrained to one marker pair:

```md
<!-- bdg:begin -->
[![crates.io](https://img.shields.io/crates/v/bdg.svg)](https://crates.io/crates/bdg)
<!-- bdg:end -->
```

If the block is absent, `add`, `sync`, and `catalog add` insert it below the first H1 heading. `check` reports missing or duplicated markers instead of repairing them.

## Configuration

`bdg` searches from the active directory up to the Git root for `.bdg.toml`.

```toml
[version]
allow_yy_calver = false
year_min = 2000
year_max = 2199

[badges]
exclude = ["release", "coverage"]

[catalog]
sources = [
  "./team-catalog.toml",
  "https://example.com/bdg-catalog.json",
]
```

An explicit `--only` overrides configured badge exclusions for that invocation. Catalog sources configured here are loaded automatically by `bdg catalog search` and `bdg catalog add`; explicit `--source` values are merged as additional sources. The project-local `.bdg/catalog.toml` file is also loaded automatically when present.

## TUI keys

- Up/Down: move
- Space: toggle
- Enter: apply
- q/Esc/Ctrl+C: cancel

## Version classification

Versions are classified as calver, semver, or unknown with calver priority. Use `--allow-yy-calver` to opt in to `YY.MM` / `YY.MM.MICRO` patterns.

## Exit codes

- `0`: success, valid state, or already synchronized
- `1`: runtime or validation error
- `2`: CLI usage error, or pending changes reported by `--dry-run` / `--check`
