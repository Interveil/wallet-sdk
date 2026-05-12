use std::sync::Mutex;
use veil_wallet_sdk::{Wallet, WalletError};

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
    let wallet = Wallet::create().unwrap();
    assert!(wallet.mnemonic().is_some(), "new wallet must have mnemonic");
    assert!(
        wallet.eth_address().starts_with("0x"),
        "eth must start with 0x"
    );
    assert_eq!(wallet.eth_address().len(), 42, "eth must be 42 chars");
    assert!(!wallet.sol_address().is_empty(), "sol must not be empty");
    assert!(
        wallet.pvx_address().starts_with("pvx1"),
        "pvx must start with pvx1"
    );
}

#[test]
fn test_import_valid_mnemonic() {
    let phrase =
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    let wallet = Wallet::import(phrase).unwrap();
    assert_eq!(wallet.eth_address().len(), 42);
    assert_ne!(wallet.sol_address().len(), 0);
}

#[test]
fn test_import_invalid_mnemonic() {
    let result = Wallet::import("foo bar baz");
    assert!(matches!(result, Err(WalletError::InvalidMnemonic)));
}

#[test]
fn test_save_load_roundtrip() {
    with_isolated_dir("cli_save_load", || {
        let wallet = Wallet::create().unwrap();
        let eth = wallet.eth_address().to_string();
        let sol = wallet.sol_address().to_string();
        let pvx = wallet.pvx_address().to_string();

        wallet.save("correct-horse-battery-staple").unwrap();

        let loaded = Wallet::load("correct-horse-battery-staple").unwrap();
        assert_eq!(eth, loaded.eth_address());
        assert_eq!(sol, loaded.sol_address());
        assert_eq!(pvx, loaded.pvx_address());
    });
}

#[test]
fn test_address_filtering() {
    with_isolated_dir("cli_address_filter", || {
        let wallet = Wallet::create().unwrap();
        wallet.save("pw").unwrap();

        let loaded = Wallet::load("pw").unwrap();
        assert_eq!(wallet.eth_address(), loaded.eth_address());
        assert_eq!(wallet.sol_address(), loaded.sol_address());
        assert_eq!(wallet.pvx_address(), loaded.pvx_address());
    });
}
