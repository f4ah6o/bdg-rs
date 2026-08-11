use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> anyhow::Result<ExitCode> {
    let cli = match bdg::cli::parse_args(std::env::args().skip(1)) {
        Ok(bdg::cli::ParseOutcome::Run(cli)) => cli,
        Ok(bdg::cli::ParseOutcome::Help) => {
            print!("{}", bdg::cli::help());
            return Ok(ExitCode::SUCCESS);
        }
        Ok(bdg::cli::ParseOutcome::Version) => {
            println!("{}", env!("CARGO_PKG_VERSION"));
            return Ok(ExitCode::SUCCESS);
        }
        Err(message) => {
            eprintln!("error: {message}\n\n{}", bdg::cli::help());
            return Ok(ExitCode::from(2));
        }
    };

    let process_dir = std::env::current_dir()?;
    let current_dir = match cli.directory {
        Some(directory) => {
            let path = PathBuf::from(directory);
            let path = if path.is_absolute() {
                path
            } else {
                process_dir.join(path)
            };
            if !path.is_dir() {
                anyhow::bail!(
                    "directory does not exist or is not a directory: {}",
                    path.display()
                );
            }
            path
        }
        None => process_dir,
    };

    let code = match cli.command {
        bdg::cli::Commands::Add {
            yes,
            only,
            allow_yy_calver,
            dry_run,
            json,
        } => bdg::app::cmd_add(&current_dir, yes, &only, allow_yy_calver, dry_run, json)?,
        bdg::cli::Commands::Sync {
            only,
            allow_yy_calver,
            dry_run,
            check,
            json,
        } => bdg::app::cmd_sync(&current_dir, &only, allow_yy_calver, dry_run || check, json)?,
        bdg::cli::Commands::Check { json, strict } => {
            bdg::check::cmd_check(&current_dir, json, strict)?
        }
        bdg::cli::Commands::List {
            json,
            quiet,
            allow_yy_calver,
        } => {
            bdg::app::cmd_list(&current_dir, json, quiet, allow_yy_calver)?;
            0
        }
        bdg::cli::Commands::Remove {
            all,
            id,
            kind,
            strict,
            quiet,
            dry_run,
            json,
            allow_yy_calver,
        } => bdg::app::cmd_remove(
            &current_dir,
            all,
            &id,
            &kind,
            strict,
            quiet,
            dry_run,
            json,
            allow_yy_calver,
        )?,
        bdg::cli::Commands::Skills => {
            bdg::app::cmd_skills()?;
            0
        }
    };

    Ok(ExitCode::from(code as u8))
}
