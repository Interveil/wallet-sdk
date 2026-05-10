use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "wallet", about = "Multichain wallet CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Create a new wallet (prints mnemonic once, does not save)
    Create,
    /// Import wallet from mnemonic phrase
    Import { mnemonic: String },
    /// Create and save a new wallet to encrypted vault
    Save {
        #[arg(long, short)]
        password: Option<String>,
    },
    /// Load wallet from encrypted vault
    Load {
        #[arg(long, short)]
        password: Option<String>,
    },
    /// Display wallet addresses (loads from vault, prompts for password)
    Address {
        #[arg(long)]
        eth: bool,
        #[arg(long)]
        sol: bool,
        #[arg(long)]
        pvx: bool,
    },
}

pub fn resolve_password(cli_pw: Option<String>) -> Result<String> {
    match cli_pw {
        Some(pw) => Ok(pw),
        None => rpassword::prompt_password("Password: ").context("failed to read password"),
    }
}

pub fn cmd_create() -> Result<()> {
    let wallet = sdk::Wallet::create().context("failed to create wallet")?;

    println!("Wallet Created\n");

    if let Some(mnemonic) = wallet.mnemonic() {
        println!("Mnemonic: {mnemonic}\n");
    }

    println!("ETH: {}", wallet.eth_address());
    println!("SOL: {}", wallet.sol_address());
    println!("PVX: {}", wallet.pvx_address());

    println!("\n\u{26a0} Save your mnemonic safely. It will NOT be shown again.");

    Ok(())
}

pub fn cmd_import(mnemonic: &str) -> Result<()> {
    let wallet = sdk::Wallet::import(mnemonic).context("invalid mnemonic phrase")?;

    println!("Wallet Imported\n");
    println!("ETH: {}", wallet.eth_address());
    println!("SOL: {}", wallet.sol_address());
    println!("PVX: {}", wallet.pvx_address());

    Ok(())
}

pub fn cmd_save(password: &str) -> Result<()> {
    let wallet = sdk::Wallet::create().context("failed to create wallet")?;
    wallet.save(password).context("failed to save wallet")?;

    println!("Wallet Created & Saved\n");

    if let Some(mnemonic) = wallet.mnemonic() {
        println!("Mnemonic: {mnemonic}\n");
    }

    println!("ETH: {}", wallet.eth_address());
    println!("SOL: {}", wallet.sol_address());
    println!("PVX: {}", wallet.pvx_address());

    println!("\n\u{26a0} Save your mnemonic safely. It will NOT be shown again.");

    Ok(())
}

pub fn cmd_load(password: &str) -> Result<()> {
    let wallet = sdk::Wallet::load(password).context("failed to load wallet")?;

    println!("Wallet Loaded\n");
    println!("ETH: {}", wallet.eth_address());
    println!("SOL: {}", wallet.sol_address());
    println!("PVX: {}", wallet.pvx_address());

    Ok(())
}

pub fn cmd_address(eth: bool, sol: bool, pvx: bool) -> Result<()> {
    let password = rpassword::prompt_password("Password: ").context("failed to read password")?;
    let wallet = sdk::Wallet::load(&password).context("failed to load wallet")?;

    let any_flag = eth || sol || pvx;
    if !any_flag || eth {
        println!("ETH: {}", wallet.eth_address());
    }
    if !any_flag || sol {
        println!("SOL: {}", wallet.sol_address());
    }
    if !any_flag || pvx {
        println!("PVX: {}", wallet.pvx_address());
    }

    Ok(())
}
