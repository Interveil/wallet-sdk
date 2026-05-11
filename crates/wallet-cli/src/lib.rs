use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use rand::seq::SliceRandom;

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
    /// Import wallet from mnemonic phrase (optionally save to vault with --save)
    Import {
        mnemonic: String,
        #[arg(long, short)]
        save: bool,
        #[arg(long, short)]
        password: Option<String>,
    },
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
    /// Export mnemonic phrase (loads from vault with password)
    ExportSeed {
        #[arg(long, short)]
        password: Option<String>,
    },
    /// Export chain private key (loads from vault with password)
    ExportPrivateKey {
        #[arg(long)]
        chain: String,
        #[arg(long, short)]
        password: Option<String>,
    },
    /// Verify mnemonic backup by typing random words
    Verify {
        #[arg(long, short)]
        password: Option<String>,
    },
}

/// Reads a password from the CLI argument or prompts interactively.
///
/// # Errors
///
/// Returns an error if the interactive password prompt fails.
pub fn resolve_password(cli_pw: Option<String>) -> Result<String> {
    match cli_pw {
        Some(pw) => Ok(pw),
        None => dialoguer::Password::new()
            .with_prompt("Password")
            .interact()
            .context("failed to read password"),
    }
}

/// Creates a new random wallet and prints the mnemonic + addresses.
///
/// Does NOT persist the wallet to disk.
///
/// # Errors
///
/// Returns an error if wallet creation or address derivation fails.
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

/// Imports a wallet from a BIP-39 mnemonic phrase and prints addresses.
///
/// Does NOT persist the wallet to disk.
///
/// # Errors
///
/// Returns an error if the mnemonic is invalid or address derivation fails.
pub fn cmd_import(mnemonic: &str, save: bool, password: &str) -> Result<()> {
    let wallet = sdk::Wallet::import(mnemonic).context("invalid mnemonic phrase")?;

    if save {
        wallet.save(password).context("failed to save wallet")?;
        println!("Wallet Imported & Saved\n");
    } else {
        println!("Wallet Imported\n");
    }

    println!("ETH: {}", wallet.eth_address());
    println!("SOL: {}", wallet.sol_address());
    println!("PVX: {}", wallet.pvx_address());

    Ok(())
}

/// Creates a new wallet, saves it to the encrypted vault, and prints details.
///
/// # Errors
///
/// Returns an error if wallet creation, encryption, or I/O fails.
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

/// Loads a wallet from the encrypted vault and prints addresses.
///
/// # Errors
///
/// Returns an error if the password is wrong, the file is corrupted,
/// or I/O fails.
pub fn cmd_load(password: &str) -> Result<()> {
    let wallet = sdk::Wallet::load(password).context("failed to load wallet")?;

    println!("Wallet Loaded\n");
    println!("ETH: {}", wallet.eth_address());
    println!("SOL: {}", wallet.sol_address());
    println!("PVX: {}", wallet.pvx_address());

    Ok(())
}

/// Loads wallet from vault and displays filtered addresses.
///
/// Prompts for password interactively. If no `--eth`/`--sol`/`--pvx` flags
/// are given, all addresses are shown.
///
/// # Errors
///
/// Returns an error if password input, wallet loading, or address
/// derivation fails.
pub fn cmd_address(eth: bool, sol: bool, pvx: bool) -> Result<()> {
    let password = dialoguer::Password::new()
        .with_prompt("Password")
        .interact()
        .context("failed to read password")?;
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

/// Exports the mnemonic seed phrase (loads from vault with password).
///
/// Prompts for confirmation before displaying the seed.
///
/// # Errors
///
/// Returns an error if the password is wrong, the vault is corrupted,
/// or user input fails.
pub fn cmd_export_seed(password: &str) -> Result<()> {
    let wallet = sdk::Wallet::load(password).context("failed to load wallet")?;

    let Some(mnemonic) = wallet.mnemonic() else {
        println!("No mnemonic available (wallet was saved without mnemonic).");
        return Ok(());
    };

    let confirmed = dialoguer::Confirm::new()
        .with_prompt("Are you sure? (yes/no)")
        .interact()
        .context("failed to read confirmation")?;

    if !confirmed {
        println!("Export cancelled.");
        return Ok(());
    }

    println!("Mnemonic: {mnemonic}");
    Ok(())
}

/// Exports a chain-specific private key (loads from vault with password).
///
/// Use `--chain eth | sol | pvx`. Prompts for confirmation before
/// displaying the key.
///
/// # Errors
///
/// Returns an error if the chain is unknown, the password is wrong,
/// the vault is corrupted, or user input fails.
pub fn cmd_export_private_key(chain: &str, password: &str) -> Result<()> {
    match chain {
        "eth" | "sol" | "pvx" => {}
        _ => anyhow::bail!("unknown chain '{chain}'. Use --chain eth | sol | pvx"),
    }

    let confirmed = dialoguer::Confirm::new()
        .with_prompt("Are you sure? (yes/no)")
        .interact()
        .context("failed to read confirmation")?;

    if !confirmed {
        println!("Export cancelled.");
        return Ok(());
    }

    let wallet = sdk::Wallet::load(password).context("failed to load wallet")?;

    match chain {
        "eth" => {
            let key = wallet
                .eth_private_key()
                .context("failed to derive ETH private key")?;
            println!("ETH Private Key: 0x{}", hex::encode(key));
        }
        "sol" => {
            let key = wallet
                .sol_private_key()
                .context("failed to derive SOL private key")?;
            let encoded = bs58::encode(key).into_string();
            println!("SOL Private Key: {encoded}");
        }
        "pvx" => {
            let key = wallet
                .pvx_spending_key()
                .context("failed to derive PVX spending key")?;
            let hrp = bech32::Hrp::parse("pvxsk").unwrap();
            let encoded =
                bech32::encode_lower::<bech32::Bech32>(hrp, key.as_ref()).unwrap();
            println!("PVX Spending Key: {encoded}");
        }
        _ => unreachable!(),
    }

    Ok(())
}

/// Verifies mnemonic backup by prompting for 3 random words.
///
/// Loads the wallet from the vault with the given password.
///
/// # Errors
///
/// Returns an error if the password is wrong, the vault is corrupted,
/// or user input fails.
pub fn cmd_verify(password: &str) -> Result<()> {
    let wallet = sdk::Wallet::load(password).context("failed to load wallet")?;

    let Some(mnemonic_str) = wallet.mnemonic() else {
        anyhow::bail!("No mnemonic available for verification (wallet was saved without mnemonic)");
    };

    let words: Vec<&str> = mnemonic_str.split_whitespace().collect();

    let mut positions: Vec<usize> = (0..words.len()).collect();
    positions.shuffle(&mut rand::thread_rng());
    let selected: Vec<usize> = positions.into_iter().take(3).collect();

    let mut correct = true;
    for &pos in &selected {
        let answer: String = dialoguer::Input::new()
            .with_prompt(format!("Word {}:", pos + 1))
            .interact_text()
            .context("failed to read input")?;

        if answer.trim() != words[pos] {
            correct = false;
        }
    }

    if correct {
        println!("Backup verified.");
    } else {
        println!("Backup verification failed.");
    }

    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use sdk::SdkError;
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
        assert!(wallet.mnemonic().is_some());
        assert!(wallet.eth_address().starts_with("0x"));
        assert_eq!(wallet.eth_address().len(), 42);
        assert!(wallet.sol_address().len() >= 32);
        assert!(wallet.pvx_address().starts_with("pvx1"));
    }

    #[test]
    fn test_import_valid_mnemonic() {
        let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let wallet = sdk::Wallet::import(phrase).unwrap();
        assert_eq!(wallet.eth_address().len(), 42);
        assert_ne!(wallet.sol_address().len(), 0);
    }

    #[test]
    fn test_import_invalid_mnemonic() {
        let result = sdk::Wallet::import("foo bar baz");
        assert!(matches!(result, Err(SdkError::InvalidMnemonic)));
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
    fn test_save_load_preserves_mnemonic() {
        with_isolated_dir("cli_save_load_mnemonic", || {
            let wallet = sdk::Wallet::create().unwrap();
            let orig_mnemonic = wallet.mnemonic().unwrap().to_string();

            wallet.save("pw").unwrap();

            let loaded = sdk::Wallet::load("pw").unwrap();
            assert_eq!(
                loaded.mnemonic(),
                Some(orig_mnemonic.as_str()),
                "mnemonic must survive save/load roundtrip"
            );
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

    #[test]
    fn test_export_invalid_chain() {
        let result = crate::cmd_export_private_key("btc", "");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("unknown chain"),
            "error must mention 'unknown chain': {err}"
        );
    }

    #[test]
    fn test_private_key_derivation() {
        let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let wallet = sdk::Wallet::import(phrase).unwrap();

        let eth_key = wallet.eth_private_key().unwrap();
        assert_eq!(eth_key.len(), 32);

        let sol_key = wallet.sol_private_key().unwrap();
        assert_eq!(sol_key.len(), 32);

        let pvx_key = wallet.pvx_spending_key().unwrap();
        assert_eq!(pvx_key.len(), 32);

        // Verify known addresses from golden test vectors
        assert_eq!(
            wallet.eth_address(),
            "0x9858effd232b4033e47d90003d41ec34ecaeda94"
        );
        assert_eq!(
            wallet.sol_address(),
            "HAgk14JpMQLgt6rVgv7cBQFJWFto5Dqxi472uT3DKpqk"
        );

        // All three keys must be different
        assert_ne!(eth_key, sol_key);
        assert_ne!(eth_key, pvx_key);
        assert_ne!(sol_key, pvx_key);
    }
}
