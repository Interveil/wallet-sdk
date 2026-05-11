use anyhow::Result;
use clap::Parser;
use wallet_cli::{
    cmd_address, cmd_create, cmd_export_private_key, cmd_export_seed, cmd_import, cmd_load,
    cmd_save, cmd_verify, resolve_password, Cli, Commands,
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
        Commands::ExportSeed { password } => cmd_export_seed(&resolve_password(password)?),
        Commands::ExportPrivateKey { chain, password } => {
            cmd_export_private_key(&chain, &resolve_password(password)?)
        }
        Commands::Verify { password } => cmd_verify(&resolve_password(password)?),
    }
}
