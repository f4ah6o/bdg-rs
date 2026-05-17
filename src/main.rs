fn main() -> anyhow::Result<std::process::ExitCode> {
    let cli = match bdg::cli::parse_args(std::env::args().skip(1)) {
        Ok(bdg::cli::ParseOutcome::Run(cli)) => cli,
        Ok(bdg::cli::ParseOutcome::Help) => {
            print!("{}", bdg::cli::help());
            return Ok(std::process::ExitCode::SUCCESS);
        }
        Ok(bdg::cli::ParseOutcome::Version) => {
            println!("{}", env!("CARGO_PKG_VERSION"));
            return Ok(std::process::ExitCode::SUCCESS);
        }
        Err(message) => {
            eprintln!("error: {message}\n\n{}", bdg::cli::help());
            return Ok(std::process::ExitCode::from(2));
        }
    };
    let current_dir = std::env::current_dir()?;
    match cli.command {
        bdg::cli::Commands::Add {
            yes,
            only,
            allow_yy_calver,
            dry_run,
            json,
        } => {
            let code = bdg::app::cmd_add(&current_dir, yes, &only, allow_yy_calver, dry_run, json)?;
            return Ok(std::process::ExitCode::from(code as u8));
        }
        bdg::cli::Commands::List {
            json,
            quiet,
            allow_yy_calver,
        } => {
            bdg::app::cmd_list(&current_dir, json, quiet, allow_yy_calver)?;
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
        } => {
            let code = bdg::app::cmd_remove(
                &current_dir,
                all,
                &id,
                &kind,
                strict,
                quiet,
                dry_run,
                json,
                allow_yy_calver,
            )?;
            return Ok(std::process::ExitCode::from(code as u8));
        }
        bdg::cli::Commands::Skills => {
            bdg::app::cmd_skills()?;
        }
    }
    Ok(std::process::ExitCode::SUCCESS)
}
