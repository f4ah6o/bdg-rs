#[derive(Debug, PartialEq, Eq)]
pub enum ParseOutcome {
    Run(Cli),
    Help,
    Version,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Cli {
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
    Skills,
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
        "skills" => Commands::Skills,
        other => return Err(format!("unknown command `{other}`")),
    };

    if let Some(arg) = args.first() {
        return Err(format!("unexpected argument `{arg}`"));
    }
    Ok(ParseOutcome::Run(Cli { command }))
}

pub fn help() -> &'static str {
    "Interactive Badge Manager CLI\n\nUsage:\n  bdg <COMMAND> [OPTIONS]\n\nCommands:\n  add       Add badges to the managed README block\n  list      List managed badges\n  remove    Remove managed badges\n  skills    Print the bundled bdg Agent Skill\n\nGlobal options:\n  -h, --help       Print help\n  -V, --version    Print version\n\nAdd options:\n      --yes\n      --only <TYPES>      Comma-separated: ci,version,license,release,docs,downloads,coverage\n      --allow-yy-calver\n      --dry-run\n      --json\n\nList options:\n      --json\n      --quiet\n      --allow-yy-calver\n\nRemove options:\n      --all\n      --id <ID>\n      --kind <KIND>\n      --strict\n      --quiet\n      --dry-run\n      --json\n      --allow-yy-calver\n"
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
    use super::{Commands, ParseOutcome, parse_args};

    #[test]
    fn parses_add_flags_and_comma_only() {
        let parsed =
            parse_args(["bdg", "add", "--yes", "--only", "ci,version", "--json"]).expect("parse");
        assert_eq!(
            parsed,
            ParseOutcome::Run(super::Cli {
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
