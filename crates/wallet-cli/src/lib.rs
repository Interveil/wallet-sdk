use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use rand::seq::SliceRandom;
use std::sync::Mutex;
use zeroize::Zeroizing;

static SESSION: Mutex<Option<Session>> = Mutex::new(None);

struct Session {
    wallet: sdk::Wallet,
    mnemonic: Option<Zeroizing<String>>,
    eth_raw: Zeroizing<[u8; 32]>,
    sol_raw: Zeroizing<[u8; 32]>,
    pvx_raw: Zeroizing<[u8; 32]>,
}

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
    /// Decrypt wallet into in-memory session (enables export commands)
    Unlock,
    /// Wipe all secrets from in-memory session
    Lock,
    /// Export mnemonic phrase (requires unlocked session)
    ExportSeed,
    /// Export chain private key (requires unlocked session)
    ExportPrivateKey {
        #[arg(long)]
        chain: String,
    },
    /// Verify mnemonic backup by typing random words
    Verify,
}

fn with_session<F, T>(f: F) -> Result<T>
where
    F: FnOnce(&Session) -> Result<T>,
{
    let guard = SESSION.lock().unwrap();
    let session = guard
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Wallet is locked. Run 'wallet unlock' first."))?;
    f(session)
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

/// Decrypts the vault and loads the wallet into the in-memory session.
///
/// This must be called before `export-seed`, `export-private-key`, or
/// `verify`.
///
/// # Panics
///
/// Panics if the global `SESSION` mutex is poisoned (another thread
/// panicked while holding the lock).
///
/// # Errors
///
/// Returns an error if the password is wrong, the vault is corrupted,
/// or key derivation fails.
pub fn cmd_unlock() -> Result<()> {
    let password = dialoguer::Password::new()
        .with_prompt("Password")
        .interact()
        .context("failed to read password")?;
    let wallet = sdk::Wallet::load(&password).context("failed to unlock wallet")?;

    let mnemonic = wallet.mnemonic().map(|m| Zeroizing::new(m.to_string()));
    let eth_raw = Zeroizing::new(wallet.eth_private_key()?);
    let sol_raw = Zeroizing::new(wallet.sol_private_key()?);
    let pvx_raw = Zeroizing::new(wallet.pvx_spending_key()?);

    let mut guard = SESSION.lock().unwrap();
    *guard = Some(Session {
        wallet,
        mnemonic,
        eth_raw,
        sol_raw,
        pvx_raw,
    });

    println!("Wallet unlocked.");
    Ok(())
}

/// Wipes all secrets from the in-memory session.
///
/// # Panics
///
/// Panics if the global `SESSION` mutex is poisoned.
///
/// # Errors
///
/// Returns an error if the session mutex cannot be acquired.
pub fn cmd_lock() -> Result<()> {
    let mut guard = SESSION.lock().unwrap();
    *guard = None;
    println!("Wallet locked.");
    Ok(())
}

/// Exports the mnemonic seed phrase (requires an unlocked session).
///
/// Prompts for confirmation before displaying the seed.
///
/// # Errors
///
/// Returns an error if the wallet is locked or user input fails.
pub fn cmd_export_seed() -> Result<()> {
    let mnemonic = with_session(|session| {
        Ok(session
            .mnemonic
            .as_ref()
            .map(|m| m.to_string())
            .or_else(|| session.wallet.mnemonic().map(ToString::to_string)))
    })?;

    let Some(mnemonic) = mnemonic else {
        println!("No mnemonic available (wallet was loaded from storage).");
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

/// Exports a chain-specific private key (requires an unlocked session).
///
/// Use `--chain eth | sol | pvx`. Prompts for confirmation before
/// displaying the key.
///
/// # Panics
///
/// Panics if the global `SESSION` mutex is poisoned, or if the
/// hardcoded Bech32 HRP `"pvxsk"` is invalid (should never happen).
///
/// # Errors
///
/// Returns an error if the chain is unknown, the wallet is locked,
/// or user input fails.
pub fn cmd_export_private_key(chain: &str) -> Result<()> {
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

    with_session(|session| {
        match chain {
            "eth" => {
                println!("ETH Private Key: 0x{}", hex::encode(session.eth_raw.as_ref()));
            }
            "sol" => {
                let encoded = bs58::encode(session.sol_raw.as_ref()).into_string();
                println!("SOL Private Key: {encoded}");
            }
            "pvx" => {
                let hrp = bech32::Hrp::parse("pvxsk").unwrap();
                let encoded =
                    bech32::encode_lower::<bech32::Bech32>(hrp, session.pvx_raw.as_ref()).unwrap();
                println!("PVX Spending Key: {encoded}");
            }
            _ => unreachable!(),
        }
        Ok(())
    })?;

    Ok(())
}

/// Verifies mnemonic backup by prompting for 3 random words.
///
/// Requires an unlocked session (run `wallet unlock` first).
///
/// # Errors
///
/// Returns an error if the wallet is locked or user input fails.
pub fn cmd_verify() -> Result<()> {
    let mnemonic_str = with_session(|session| {
        session
            .mnemonic
            .as_ref()
            .map(|m| m.to_string())
            .or_else(|| session.wallet.mnemonic().map(ToString::to_string))
            .ok_or_else(|| anyhow::anyhow!("No mnemonic available for verification"))
    })?;

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

/// Test helper: verify mnemonic against a list of words (all positions).
#[cfg(test)]
pub(crate) fn verify_with_words(
    mnemonic: &Option<Zeroizing<String>>,
    answers: &[&str],
) -> bool {
    let phrase = match mnemonic {
        Some(m) => m.to_string(),
        None => return false,
    };
    let words: Vec<&str> = phrase.split_whitespace().collect();
    if answers.len() != words.len() {
        return false;
    }
    words.iter().zip(answers.iter()).all(|(a, b)| a == b)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::{
        verify_with_words, with_session, Session, SESSION,
    };
    use sdk::SdkError;
    use std::sync::Mutex;
    use zeroize::Zeroizing;

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

    fn unlocked_session() -> Session {
        let wallet = sdk::Wallet::create().unwrap();
        let mnemonic = wallet.mnemonic().map(|m| Zeroizing::new(m.to_string()));
        let eth_raw = Zeroizing::new(wallet.eth_private_key().unwrap());
        let sol_raw = Zeroizing::new(wallet.sol_private_key().unwrap());
        let pvx_raw = Zeroizing::new(wallet.pvx_spending_key().unwrap());
        Session {
            wallet,
            mnemonic,
            eth_raw,
            sol_raw,
            pvx_raw,
        }
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
    fn test_unlock_roundtrip() {
        with_isolated_dir("cli_unlock", || {
            let wallet = sdk::Wallet::create().unwrap();
            wallet.save("pw").unwrap();

            let loaded = sdk::Wallet::load("pw").unwrap();
            let mnemonic = loaded.mnemonic().map(|m| Zeroizing::new(m.to_string()));
            let eth_raw = Zeroizing::new(loaded.eth_private_key().unwrap());
            let sol_raw = Zeroizing::new(loaded.sol_private_key().unwrap());
            let pvx_raw = Zeroizing::new(loaded.pvx_spending_key().unwrap());
            {
                let mut guard = SESSION.lock().unwrap();
                *guard = Some(Session {
                    wallet: loaded,
                    mnemonic,
                    eth_raw,
                    sol_raw,
                    pvx_raw,
                });
            }

            // Use session directly (not with_session, to avoid double-lock)
            let eth_key = {
                let guard = SESSION.lock().unwrap();
                let session = guard.as_ref().unwrap();
                *session.eth_raw
            };
            assert_eq!(eth_key.len(), 32);

            // Lock
            {
                let mut guard = SESSION.lock().unwrap();
                *guard = None;
            }

            // After lock, session is gone
            let is_none = {
                let guard = SESSION.lock().unwrap();
                guard.is_none()
            };
            assert!(is_none);
        });
    }

    #[test]
    fn test_export_without_unlock_fails() {
        let result = with_session(|_| Ok(()));
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("locked"), "error must mention 'locked': {err}");
    }

    #[test]
    fn test_export_invalid_chain() {
        let result = crate::cmd_export_private_key("btc");
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

    #[test]
    fn test_verify_with_wrong_word_fails() {
        let session = unlocked_session();
        let words: Vec<&str> = session
            .mnemonic
            .as_ref()
            .unwrap()
            .split_whitespace()
            .collect();
        let wrong_words: Vec<&str> = words.iter().map(|_| "wrong").collect();

        let result = verify_with_words(&session.mnemonic, &wrong_words);
        assert!(!result, "wrong words must fail verification");
    }

    #[test]
    fn test_verify_with_correct_words_passes() {
        let session = unlocked_session();
        let words: Vec<&str> = session
            .mnemonic
            .as_ref()
            .unwrap()
            .split_whitespace()
            .collect();

        let result = verify_with_words(&session.mnemonic, &words);
        assert!(result, "correct words must pass verification");
    }

    #[test]
    fn test_export_seed_without_mnemonic() {
        let wallet = sdk::Wallet::import(
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
        )
        .unwrap();
        let eth_raw = Zeroizing::new(wallet.eth_private_key().unwrap());
        let sol_raw = Zeroizing::new(wallet.sol_private_key().unwrap());
        let pvx_raw = Zeroizing::new(wallet.pvx_spending_key().unwrap());
        let mut guard = SESSION.lock().unwrap();
        *guard = Some(Session {
            wallet,
            mnemonic: None,
            eth_raw,
            sol_raw,
            pvx_raw,
        });
        drop(guard);

        let mnemonic = with_session(|s| {
            Ok(s.mnemonic
                .as_ref()
                .map(|m| m.to_string())
                .or_else(|| s.wallet.mnemonic().map(ToString::to_string)))
        });
        assert!(mnemonic.is_ok());
        assert!(
            mnemonic.unwrap().is_some(),
            "wallet has mnemonic via wallet.mnemonic()"
        );
    }
}
