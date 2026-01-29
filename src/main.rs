use clap::Parser;

fn main() -> anyhow::Result<std::process::ExitCode> {
    let cli = bdg::cli::Cli::parse();
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
    }
    Ok(std::process::ExitCode::SUCCESS)
}
