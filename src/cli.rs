#[derive(Debug, PartialEq, Eq)]
pub enum ParseOutcome {
    Run(Cli),
    Help,
    Version,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Cli {
    pub directory: Option<String>,
    pub command: Commands,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Commands {
    Add {
        yes: bool,
        only: Vec<String>,
        allow_yy_calver: bool,
        dry_run: bool,
        json: bool,
    },
    Sync {
        only: Vec<String>,
        allow_yy_calver: bool,
        dry_run: bool,
        check: bool,
        json: bool,
    },
    Check {
        json: bool,
        strict: bool,
    },
    List {
        json: bool,
        quiet: bool,
        allow_yy_calver: bool,
    },
    Remove {
        all: bool,
        id: Vec<String>,
        kind: Vec<String>,
        strict: bool,
        quiet: bool,
        dry_run: bool,
        json: bool,
        allow_yy_calver: bool,
    },
    Catalog(CatalogCommand),
    Skills,
}

#[derive(Debug, PartialEq, Eq)]
pub enum CatalogCommand {
    Search {
        query: Option<String>,
        source: Vec<String>,
        json: bool,
    },
    Add {
        ids: Vec<String>,
        source: Vec<String>,
        set: Vec<String>,
        dry_run: bool,
        json: bool,
    },
    AddUrl {
        image: String,
        label: String,
        link: Option<String>,
        dry_run: bool,
        json: bool,
    },
}

pub fn parse_args<I, S>(args: I) -> Result<ParseOutcome, String>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut args: Vec<String> = args.into_iter().map(Into::into).collect();
    if args.first().is_some_and(|arg| arg == "bdg") {
        args.remove(0);
    }
    if args.is_empty() {
        return Err("missing command".to_string());
    }
    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        return Ok(ParseOutcome::Help);
    }
    if args.iter().any(|arg| arg == "-V" || arg == "--version") {
        return Ok(ParseOutcome::Version);
    }

    let directory = take_single_value(&mut args, &["-C", "--directory"])?;
    if args.is_empty() {
        return Err("missing command".to_string());
    }

    let command = args.remove(0);
    let command = match command.as_str() {
        "add" => Commands::Add {
            yes: take_bool(&mut args, "--yes")?,
            only: take_values(&mut args, "--only")?
                .into_iter()
                .flat_map(|value| split_csv(&value))
                .collect(),
            allow_yy_calver: take_bool(&mut args, "--allow-yy-calver")?,
            dry_run: take_bool(&mut args, "--dry-run")?,
            json: take_bool(&mut args, "--json")?,
        },
        "sync" => Commands::Sync {
            only: take_values(&mut args, "--only")?
                .into_iter()
                .flat_map(|value| split_csv(&value))
                .collect(),
            allow_yy_calver: take_bool(&mut args, "--allow-yy-calver")?,
            dry_run: take_bool(&mut args, "--dry-run")?,
            check: take_bool(&mut args, "--check")?,
            json: take_bool(&mut args, "--json")?,
        },
        "check" => Commands::Check {
            json: take_bool(&mut args, "--json")?,
            strict: take_bool(&mut args, "--strict")?,
        },
        "list" => Commands::List {
            json: take_bool(&mut args, "--json")?,
            quiet: take_bool(&mut args, "--quiet")?,
            allow_yy_calver: take_bool(&mut args, "--allow-yy-calver")?,
        },
        "remove" => Commands::Remove {
            all: take_bool(&mut args, "--all")?,
            id: take_values(&mut args, "--id")?,
            kind: take_values(&mut args, "--kind")?,
            strict: take_bool(&mut args, "--strict")?,
            quiet: take_bool(&mut args, "--quiet")?,
            dry_run: take_bool(&mut args, "--dry-run")?,
            json: take_bool(&mut args, "--json")?,
            allow_yy_calver: take_bool(&mut args, "--allow-yy-calver")?,
        },
        "catalog" => Commands::Catalog(parse_catalog_command(&mut args)?),
        "skills" => Commands::Skills,
        other => return Err(format!("unknown command `{other}`")),
    };

    if let Some(arg) = args.first() {
        return Err(format!("unexpected argument `{arg}`"));
    }
    Ok(ParseOutcome::Run(Cli { directory, command }))
}

fn parse_catalog_command(args: &mut Vec<String>) -> Result<CatalogCommand, String> {
    if args.is_empty() {
        return Err("catalog requires a subcommand: search or add".to_string());
    }
    let subcommand = args.remove(0);
    match subcommand.as_str() {
        "search" => {
            let source = take_values(args, "--source")?;
            let json = take_bool(args, "--json")?;
            if args.len() > 1 {
                return Err("catalog search accepts at most one QUERY".to_string());
            }
            let query = args.pop();
            Ok(CatalogCommand::Search {
                query,
                source,
                json,
            })
        }
        "add" => {
            let source = take_values(args, "--source")?;
            let set = take_values(args, "--set")?;
            let dry_run = take_bool(args, "--dry-run")?;
            let json = take_bool(args, "--json")?;
            if args.iter().any(|arg| arg.starts_with('-')) {
                return Err(format!(
                    "unexpected argument `{}`",
                    args.iter().find(|arg| arg.starts_with('-')).unwrap()
                ));
            }
            let ids = std::mem::take(args)
                .into_iter()
                .flat_map(|value| split_csv(&value))
                .collect();
            Ok(CatalogCommand::Add {
                ids,
                source,
                set,
                dry_run,
                json,
            })
        }
        "add-url" => {
            let label =
                take_single_value(args, &["--label"])?.unwrap_or_else(|| "badge".to_string());
            let link = take_single_value(args, &["--link"])?;
            let dry_run = take_bool(args, "--dry-run")?;
            let json = take_bool(args, "--json")?;
            if args.len() != 1 {
                return Err("catalog add-url requires exactly one IMAGE_URL".to_string());
            }
            Ok(CatalogCommand::AddUrl {
                image: args.remove(0),
                label,
                link,
                dry_run,
                json,
            })
        }
        other => Err(format!("unknown catalog subcommand `{other}`")),
    }
}

pub fn help() -> &'static str {
    "Badge management for project READMEs\n\nUsage:\n  bdg <COMMAND> [OPTIONS]\n  bdg [GLOBAL OPTIONS] <COMMAND> [OPTIONS]\n\nCommands:\n  sync      Reconcile the managed badge block non-interactively\n  check     Validate marker structure and managed badge syntax\n  add       Add built-in badges to the managed README block\n  catalog   Search and add declarative badges from built-in/external catalogs\n  list      Inspect project metadata and managed badges\n  remove    Remove managed badges\n  skills    Print the bundled bdg Agent Skill\n\nGlobal options:\n  -C, --directory <PATH>  Run as if bdg started in PATH\n  -h, --help              Print help\n  -V, --version           Print version\n\nBadge types:\n  ci, version, license, release, docs, downloads, coverage,\n  msrv, stars, forks, issues, pulls, activity\n\nCatalog:\n  bdg catalog search [QUERY] [--source <PATH|URL>] [--json]\n  bdg catalog add <ID>... [--source <PATH|URL>] [--set KEY=VALUE] [--dry-run] [--json]\n  bdg catalog add-url <IMAGE_URL> [--label <TEXT>] [--link <URL>] [--dry-run] [--json]\n\n  Sources may be TOML or JSON using schema bdg.catalog/v1.\n  Project placeholders: {owner}, {repo}, {crate}, {package}, {module}, {name}.\n\nSync options:\n      --only <TYPES>      Comma-separated badge types\n      --allow-yy-calver\n      --dry-run           Print planned changes without writing\n      --check             Exit 2 when the README is not synchronized\n      --json\n\nCheck options:\n      --strict            Treat unknown managed lines as errors\n      --json\n\nAdd options:\n      --yes\n      --only <TYPES>      Comma-separated badge types\n      --allow-yy-calver\n      --dry-run\n      --json\n\nList options:\n      --json\n      --quiet\n      --allow-yy-calver\n\nRemove options:\n      --all\n      --id <ID>\n      --kind <KIND>\n      --strict\n      --quiet\n      --dry-run\n      --json\n      --allow-yy-calver\n\nExit codes:\n  0  success / synchronized\n  1  runtime or validation error\n  2  usage error or changes detected by --dry-run/--check\n"
}

fn take_bool(args: &mut Vec<String>, name: &str) -> Result<bool, String> {
    let mut found = false;
    let mut idx = 0;
    while idx < args.len() {
        if args[idx] == name {
            found = true;
            args.remove(idx);
        } else if args[idx].starts_with(&format!("{name}=")) {
            return Err(format!("`{name}` does not take a value"));
        } else {
            idx += 1;
        }
    }
    Ok(found)
}

fn take_values(args: &mut Vec<String>, name: &str) -> Result<Vec<String>, String> {
    let mut values = Vec::new();
    let mut idx = 0;
    while idx < args.len() {
        if args[idx] == name {
            args.remove(idx);
            if idx >= args.len() || args[idx].starts_with('-') {
                return Err(format!("`{name}` requires a value"));
            }
            values.push(args.remove(idx));
        } else if let Some(value) = args[idx].strip_prefix(&format!("{name}=")) {
            if value.is_empty() {
                return Err(format!("`{name}` requires a value"));
            }
            values.push(value.to_string());
            args.remove(idx);
        } else {
            idx += 1;
        }
    }
    Ok(values)
}

fn take_single_value(args: &mut Vec<String>, names: &[&str]) -> Result<Option<String>, String> {
    let mut values = Vec::new();
    for name in names {
        values.extend(take_values(args, name)?);
    }
    match values.len() {
        0 => Ok(None),
        1 => Ok(values.pop()),
        _ => Err(format!("`{}` may only be specified once", names.join("/"))),
    }
}

fn split_csv(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToString::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{CatalogCommand, Commands, ParseOutcome, parse_args};

    #[test]
    fn parses_add_flags_and_comma_only() {
        let parsed =
            parse_args(["bdg", "add", "--yes", "--only", "ci,version", "--json"]).expect("parse");
        assert_eq!(
            parsed,
            ParseOutcome::Run(super::Cli {
                directory: None,
                command: Commands::Add {
                    yes: true,
                    only: vec!["ci".to_string(), "version".to_string()],
                    allow_yy_calver: false,
                    dry_run: false,
                    json: true,
                }
            })
        );
    }

    #[test]
    fn parses_catalog_commands() {
        assert_eq!(
            parse_args([
                "catalog",
                "search",
                "github",
                "--source",
                "extra.toml",
                "--json"
            ])
            .unwrap(),
            ParseOutcome::Run(super::Cli {
                directory: None,
                command: Commands::Catalog(CatalogCommand::Search {
                    query: Some("github".to_string()),
                    source: vec!["extra.toml".to_string()],
                    json: true,
                }),
            })
        );
        assert_eq!(
            parse_args([
                "catalog",
                "add",
                "custom",
                "github-discussions,github-contributors",
                "--set",
                "value=ok",
                "--dry-run"
            ])
            .unwrap(),
            ParseOutcome::Run(super::Cli {
                directory: None,
                command: Commands::Catalog(CatalogCommand::Add {
                    ids: vec![
                        "custom".to_string(),
                        "github-discussions".to_string(),
                        "github-contributors".to_string()
                    ],
                    source: Vec::new(),
                    set: vec!["value=ok".to_string()],
                    dry_run: true,
                    json: false,
                }),
            })
        );
        assert_eq!(
            parse_args([
                "catalog",
                "add-url",
                "https://example.com/status.svg",
                "--label",
                "status",
                "--link",
                "https://example.com",
                "--dry-run"
            ])
            .unwrap(),
            ParseOutcome::Run(super::Cli {
                directory: None,
                command: Commands::Catalog(CatalogCommand::AddUrl {
                    image: "https://example.com/status.svg".to_string(),
                    label: "status".to_string(),
                    link: Some("https://example.com".to_string()),
                    dry_run: true,
                    json: false,
                }),
            })
        );
    }

    #[test]
    fn parses_sync_check_and_directory() {
        let parsed = parse_args([
            "-C",
            "repo",
            "sync",
            "--only=ci,license",
            "--check",
            "--json",
        ])
        .expect("parse");
        assert_eq!(
            parsed,
            ParseOutcome::Run(super::Cli {
                directory: Some("repo".to_string()),
                command: Commands::Sync {
                    only: vec!["ci".to_string(), "license".to_string()],
                    allow_yy_calver: false,
                    dry_run: false,
                    check: true,
                    json: true,
                }
            })
        );
        assert_eq!(
            parse_args(["check", "--strict"]).unwrap(),
            ParseOutcome::Run(super::Cli {
                directory: None,
                command: Commands::Check {
                    json: false,
                    strict: true,
                }
            })
        );
    }

    #[test]
    fn parses_equals_values_and_repeated_remove_filters() {
        let parsed = parse_args([
            "remove",
            "--id=ci:rust.yaml",
            "--id",
            "npm:bdg",
            "--kind=github_actions",
            "--strict",
        ])
        .expect("parse");
        assert_eq!(
            parsed,
            ParseOutcome::Run(super::Cli {
                directory: None,
                command: Commands::Remove {
                    all: false,
                    id: vec!["ci:rust.yaml".to_string(), "npm:bdg".to_string()],
                    kind: vec!["github_actions".to_string()],
                    strict: true,
                    quiet: false,
                    dry_run: false,
                    json: false,
                    allow_yy_calver: false,
                }
            })
        );
    }

    #[test]
    fn parses_list_flags() {
        let parsed = parse_args(["list", "--json", "--quiet", "--allow-yy-calver"]).expect("parse");
        assert_eq!(
            parsed,
            ParseOutcome::Run(super::Cli {
                directory: None,
                command: Commands::List {
                    json: true,
                    quiet: true,
                    allow_yy_calver: true,
                }
            })
        );
    }

    #[test]
    fn parses_help_and_version() {
        assert_eq!(parse_args(["--help"]).unwrap(), ParseOutcome::Help);
        assert_eq!(parse_args(["bdg", "-V"]).unwrap(), ParseOutcome::Version);
    }

    #[test]
    fn rejects_unknown_command_and_flag() {
        assert!(
            parse_args(["unknown"])
                .unwrap_err()
                .contains("unknown command")
        );
        assert!(
            parse_args(["add", "--unknown"])
                .unwrap_err()
                .contains("unexpected argument")
        );
        assert!(
            parse_args(["catalog", "nope"])
                .unwrap_err()
                .contains("unknown catalog subcommand")
        );
    }

    #[test]
    fn rejects_missing_value_and_bool_value() {
        assert!(
            parse_args(["remove", "--id"])
                .unwrap_err()
                .contains("requires a value")
        );
        assert!(
            parse_args(["list", "--json=true"])
                .unwrap_err()
                .contains("does not take a value")
        );
    }
}
