use std::sync::Mutex;
use sdk::{SdkError, Wallet};

static SERIAL: Mutex<()> = Mutex::new(());

fn with_isolated_dir<F>(test_name: &str, f: F)
where
    F: FnOnce(),
{
    let _guard = SERIAL.lock().unwrap();
    let dir = std::env::temp_dir().join(test_name);
    std::fs::create_dir_all(&dir).unwrap();
    let original = std::env::current_dir().unwrap();
    std::env::set_current_dir(&dir).unwrap();

    f();

    std::env::set_current_dir(&original).unwrap();
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_create_valid_addresses() {
    let wallet = Wallet::create().unwrap();
    let eth = wallet.eth_address();
    let sol = wallet.sol_address();
    let pvx = wallet.pvx_address();

    assert!(eth.starts_with("0x"), "eth must start with 0x");
    assert_eq!(eth.len(), 42, "eth must be 42 chars");

    assert!(!sol.is_empty(), "sol must not be empty");

    assert!(pvx.starts_with("pvx1"), "pvx must start with pvx1");
}

#[test]
fn test_import_deterministic() {
    let phrase =
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    let a = Wallet::import(phrase).unwrap();
    let b = Wallet::import(phrase).unwrap();

    assert_eq!(a.eth_address(), b.eth_address());
    assert_eq!(a.sol_address(), b.sol_address());
    assert_eq!(a.pvx_address(), b.pvx_address());
}

#[test]
fn test_different_seed_different_addresses() {
    let a = Wallet::import(
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
    )
    .unwrap();
    let b = Wallet::import(
        "legal winner thank year wave sausage worth useful legal winner thank yellow",
    )
    .unwrap();

    assert_ne!(a.eth_address(), b.eth_address());
    assert_ne!(a.sol_address(), b.sol_address());
    assert_ne!(a.pvx_address(), b.pvx_address());
}

#[test]
fn test_invalid_mnemonic_fails() {
    let result = Wallet::import("foo bar baz");
    assert!(matches!(result, Err(SdkError::InvalidMnemonic)));
}

#[test]
fn test_save_load_roundtrip() {
    with_isolated_dir("sdk_roundtrip", || {
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
fn test_wrong_password_fails_load() {
    with_isolated_dir("sdk_wrong_pw", || {
        let wallet = Wallet::create().unwrap();
        wallet.save("correct-password").unwrap();

        let result = Wallet::load("wrong-password");
        assert!(matches!(result, Err(SdkError::InvalidPassword)));
    });
}
