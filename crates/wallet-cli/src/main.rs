use anyhow::Result;
use clap::Parser;
use wallet_cli::{Cli, Commands, resolve_password, cmd_create, cmd_import, cmd_save, cmd_load, cmd_address};

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Create => cmd_create(),
        Commands::Import { mnemonic } => cmd_import(&mnemonic),
        Commands::Save { password } => cmd_save(&resolve_password(password)?),
        Commands::Load { password } => cmd_load(&resolve_password(password)?),
        Commands::Address { eth, sol, pvx } => cmd_address(eth, sol, pvx),
    }
}
