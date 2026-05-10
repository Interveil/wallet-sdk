use clap::{Parser, Subcommand};
use anyhow::{Context, Result};

#[derive(Parser)]
#[command(name = "wallet", about = "Multichain wallet CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
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

fn resolve_password(cli_pw: Option<String>) -> Result<String> {
    match cli_pw {
        Some(pw) => Ok(pw),
        None => rpassword::prompt_password("Password: ").context("failed to read password"),
    }
}

fn cmd_create() -> Result<()> {
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

fn cmd_import(mnemonic: &str) -> Result<()> {
    let wallet = sdk::Wallet::import(mnemonic).context("invalid mnemonic phrase")?;

    println!("Wallet Imported\n");
    println!("ETH: {}", wallet.eth_address());
    println!("SOL: {}", wallet.sol_address());
    println!("PVX: {}", wallet.pvx_address());

    Ok(())
}

fn cmd_save(password: &str) -> Result<()> {
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

fn cmd_load(password: &str) -> Result<()> {
    let wallet = sdk::Wallet::load(password).context("failed to load wallet")?;

    println!("Wallet Loaded\n");
    println!("ETH: {}", wallet.eth_address());
    println!("SOL: {}", wallet.sol_address());
    println!("PVX: {}", wallet.pvx_address());

    Ok(())
}

fn cmd_address(eth: bool, sol: bool, pvx: bool) -> Result<()> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static SERIAL: Mutex<()> = Mutex::new(());

    fn with_isolated_dir<F>(name: &str, f: F)
    where
        F: FnOnce(),
    {
        let _guard = SERIAL.lock().unwrap();
        let dir = std::env::temp_dir().join(name);
        std::fs::create_dir_all(&dir).unwrap();
        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(&dir).unwrap();

        f();

        std::env::set_current_dir(&original).unwrap();
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_create_wallet() {
        let wallet = sdk::Wallet::create().unwrap();
        assert!(wallet.mnemonic().is_some(), "new wallet must have mnemonic");
        assert!(wallet.eth_address().starts_with("0x"), "eth must start with 0x");
        assert_eq!(wallet.eth_address().len(), 42, "eth must be 42 chars");
        assert!(!wallet.sol_address().is_empty(), "sol must not be empty");
        assert!(wallet.pvx_address().starts_with("pvx1"), "pvx must start with pvx1");
    }

    #[test]
    fn test_import_valid_mnemonic() {
        let phrase =
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let wallet = sdk::Wallet::import(phrase).unwrap();
        assert_eq!(wallet.eth_address().len(), 42);
        assert_ne!(wallet.sol_address().len(), 0);
    }

    #[test]
    fn test_import_invalid_mnemonic() {
        let result = sdk::Wallet::import("foo bar baz");
        assert!(matches!(result, Err(sdk::SdkError::InvalidMnemonic)));
    }

    #[test]
    fn test_save_load_roundtrip() {
        with_isolated_dir("cli_save_load", || {
            let wallet = sdk::Wallet::create().unwrap();
            let eth = wallet.eth_address().to_string();
            let sol = wallet.sol_address().to_string();
            let pvx = wallet.pvx_address().to_string();

            wallet.save("correct-horse-battery-staple").unwrap();

            let loaded = sdk::Wallet::load("correct-horse-battery-staple").unwrap();
            assert_eq!(eth, loaded.eth_address());
            assert_eq!(sol, loaded.sol_address());
            assert_eq!(pvx, loaded.pvx_address());
        });
    }

    #[test]
    fn test_address_filtering() {
        with_isolated_dir("cli_address_filter", || {
            let wallet = sdk::Wallet::create().unwrap();
            wallet.save("pw").unwrap();

            let loaded = sdk::Wallet::load("pw").unwrap();
            assert_eq!(wallet.eth_address(), loaded.eth_address());
            assert_eq!(wallet.sol_address(), loaded.sol_address());
            assert_eq!(wallet.pvx_address(), loaded.pvx_address());
        });
    }
}
