use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "bdg", version, about = "Interactive Badge Manager CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Add {
        #[arg(long, default_value_t = false)]
        yes: bool,
        #[arg(long, value_delimiter = ',', value_name = "TYPES")]
        only: Vec<String>,
        #[arg(long, default_value_t = false)]
        allow_yy_calver: bool,
        #[arg(long, default_value_t = false)]
        dry_run: bool,
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    List {
        #[arg(long, default_value_t = false)]
        json: bool,
        #[arg(long, default_value_t = false)]
        quiet: bool,
        #[arg(long, default_value_t = false)]
        allow_yy_calver: bool,
    },
    Remove {
        #[arg(long, default_value_t = false)]
        all: bool,
        #[arg(long, value_name = "ID")]
        id: Vec<String>,
        #[arg(long, value_name = "KIND")]
        kind: Vec<String>,
        #[arg(long, default_value_t = false)]
        strict: bool,
        #[arg(long, default_value_t = false)]
        quiet: bool,
        #[arg(long, default_value_t = false)]
        dry_run: bool,
        #[arg(long, default_value_t = false)]
        json: bool,
        #[arg(long, default_value_t = false)]
        allow_yy_calver: bool,
    },
}
