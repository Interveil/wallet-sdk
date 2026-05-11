use anyhow::Result;
use clap::Parser;
use wallet_cli::{
    cmd_address, cmd_create, cmd_export_private_key, cmd_export_seed, cmd_import, cmd_load,
    cmd_lock, cmd_save, cmd_unlock, cmd_verify, resolve_password, Cli, Commands,
};

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Create => cmd_create(),
        Commands::Import { mnemonic, save, password } => {
            let pw = if save {
                Some(resolve_password(password)?)
            } else {
                None
            };
            cmd_import(&mnemonic, save, pw.as_deref().unwrap_or(""))
        }
        Commands::Save { password } => cmd_save(&resolve_password(password)?),
        Commands::Load { password } => cmd_load(&resolve_password(password)?),
        Commands::Address { eth, sol, pvx } => cmd_address(eth, sol, pvx),
        Commands::Unlock => cmd_unlock(),
        Commands::Lock => cmd_lock(),
        Commands::ExportSeed => cmd_export_seed(),
        Commands::ExportPrivateKey { chain } => cmd_export_private_key(&chain),
        Commands::Verify => cmd_verify(),
    }
}
