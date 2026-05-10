use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use storage::{StorageError, load_wallet, save_wallet};

// Must match storage::HEADER_LEN (1 + 16 + 12)
const HEADER_LEN: usize = 29;
use wallet_core::Wallet;

static TEST_CTR: AtomicU32 = AtomicU32::new(0);

fn test_path(name: &str) -> PathBuf {
    let n = TEST_CTR.fetch_add(1, Ordering::SeqCst);
    std::env::temp_dir().join(format!("test_{}_{}.dat", name, n))
}

#[test]
fn test_save_load_roundtrip() {
    let wallet = Wallet::create().unwrap();
    let path = test_path("roundtrip");
    let password = "correct-horse-battery-staple";

    save_wallet(&path, &wallet, password).unwrap();
    let loaded = load_wallet(&path, password).unwrap();

    assert_eq!(wallet.seed(), loaded.seed());
    assert!(loaded.mnemonic().is_empty());

    let _ = fs::remove_file(&path);
}

#[test]
fn test_wrong_password_fails() {
    let wallet = Wallet::create().unwrap();
    let path = test_path("wrong_pw");

    save_wallet(&path, &wallet, "correct-password").unwrap();
    let result = load_wallet(&path, "wrong-password");

    assert!(matches!(result, Err(StorageError::DecryptionFailed)));

    let _ = fs::remove_file(&path);
}

#[test]
fn test_corrupted_ciphertext_fails() {
    let wallet = Wallet::create().unwrap();
    let path = test_path("corrupt");

    save_wallet(&path, &wallet, "password").unwrap();

    let mut data = fs::read(&path).unwrap();
    // Flip a byte in the ciphertext area
    data[HEADER_LEN + 5] ^= 0xff;
    fs::write(&path, &data).unwrap();

    let result = load_wallet(&path, "password");
    assert!(matches!(result, Err(StorageError::DecryptionFailed)));

    let _ = fs::remove_file(&path);
}

#[test]
fn test_format_structure() {
    let wallet = Wallet::create().unwrap();
    let path = test_path("format");

    save_wallet(&path, &wallet, "password").unwrap();

    let data = fs::read(&path).unwrap();

    assert_eq!(data[0], 0x01, "version byte must be 0x01");
    assert!(
        data.len() > HEADER_LEN,
        "file must contain ciphertext after header"
    );
    // Salt and nonce should be non-zero (randomly generated)
    assert_ne!(&data[1..17], &[0u8; 16], "salt must not be zero");
    assert_ne!(&data[17..29], &[0u8; 12], "nonce must not be zero");

    let _ = fs::remove_file(&path);
}
