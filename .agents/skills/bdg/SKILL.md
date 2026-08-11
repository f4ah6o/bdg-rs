---
name: bdg
description: Use bdg to validate, synchronize, inspect, and remove README badges safely in Rust, Node, and MoonBit repositories.
---

# bdg

Use `bdg` when README badges should be managed as project metadata instead of hand-edited Markdown.

## Preferred workflow

For deterministic automation and CI, prefer:

```bash
bdg check
bdg sync --check
bdg sync
```

- `bdg check` validates the existing managed block without network-dependent discovery or writes.
- `bdg sync --check` plans the canonical detected badge set, does not write, and exits `2` when synchronization is needed.
- `bdg sync` applies that plan and only changes the managed marker block.

Use `bdg add` when a human should interactively choose a broader subset of candidates.

## Badge types

Canonical `sync` candidates include:

- `version`: npm, crates.io, MoonBit
- `ci`: detected GitHub Actions workflows
- `license`: manifest or GitHub license
- `release`: GitHub release
- `docs`: docs.rs or package documentation URL
- `downloads`: npm or crates.io downloads
- `coverage`: detected Codecov usage

Additional supported candidates are deliberately opt-in for `sync`:

- `msrv`: crates.io MSRV
- `downloads`: total GitHub release downloads
- `stars`: GitHub stars
- `forks`: GitHub forks
- `issues`: open GitHub issues
- `pulls`: open GitHub pull requests
- `activity`: GitHub last commit

An unqualified `bdg sync` does not introduce these optional repository/community badges. Use interactive `bdg add`, `bdg add --yes`, or an explicit `sync --only` when they are wanted.

```bash
bdg sync --only msrv,stars,issues,activity
bdg add --only downloads,stars,forks,pulls
```

## Commands

### `bdg sync`

Non-interactively reconciles the managed block from detected project metadata.

```bash
bdg sync
bdg sync --only ci,version,license,release,docs,downloads,coverage
bdg sync --only msrv,stars,forks,issues,pulls,activity
bdg sync --check
bdg sync --dry-run
bdg sync --json --check
```

Behavior:

- detects project metadata from `Cargo.toml`, `package.json`, or `moon.mod.json`
- detects GitHub Actions workflows from `.github/workflows`
- generates supported badge candidates from project and repository metadata
- keeps optional repository/community signals out of default `sync`
- honors `.bdg.toml` badge exclusions unless `--only` is explicit
- de-duplicates equivalent candidates
- writes only inside `<!-- bdg:begin -->` / `<!-- bdg:end -->`
- inserts the marker block if it is absent
- `--check` and `--dry-run` never write and exit `2` when a change is pending

### `bdg check`

Statically validates the existing managed block.

```bash
bdg check
bdg check --strict
bdg check --json
```

Checks include:

- exactly one ordered marker pair
- recognized badge Markdown inside the block
- duplicate stable badge ids
- strict handling of unknown managed lines with `--strict`

JSON output uses schema `bdg.check/v1`.

### `bdg add`

Interactive/manual candidate selection.

```bash
bdg add
bdg add --yes
bdg add --only ci,version,license
bdg add --only msrv,stars,forks,issues,pulls,activity
bdg add --dry-run
bdg add --json --dry-run
```

For unattended canonical synchronization, use `bdg sync` instead of `bdg add --yes`.

### `bdg list`

Reads the actual README state and detected project context.

```bash
bdg list
bdg list --json
bdg list --quiet
```

It reports repository, manifest, registry, CI, marker, and managed badge information. It is read-only and does not synthesize a missing marker block.

### `bdg remove`

Removes managed badges interactively or by stable id/kind.

```bash
bdg remove
bdg remove --id ci:rust.yaml
bdg remove --kind github_actions
bdg remove --all
bdg remove --dry-run
bdg remove --json --dry-run
```

### `bdg skills`

Prints this Agent Skills document to stdout.

## Directory targeting

All commands accept `-C/--directory` so automation does not need to mutate shell state:

```bash
bdg -C ../project check
bdg --directory ../project sync --check
bdg -C ../project list --json
```

Config discovery starts from that requested directory and stops at its Git root.

## Constraints

- Writes are limited to the bdg marker block.
- `check`, `list`, and `skills` are read-only.
- `sync --check` and all `--dry-run` operations are read-only.
- Prefer JSON output when another tool or agent will consume results.

## Project detection

- Rust: `Cargo.toml`
- Node: `package.json`
- MoonBit: `moon.mod.json`

`bdg` chooses the closest matching manifest inside the repository and supports workspace/monorepo discovery.

## Exit codes

- `0`: success, valid state, or already synchronized
- `1`: runtime or validation error
- `2`: usage error or pending changes from `--dry-run` / `--check`
