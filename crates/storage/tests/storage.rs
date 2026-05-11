use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use storage::{StorageError, load_wallet, save_wallet};

// Must match storage::HEADER_LEN (1 + 16 + 12)
const HEADER_LEN: usize = 29;

static TEST_CTR: AtomicU32 = AtomicU32::new(0);

const TEST_MNEMONIC: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

fn test_path(name: &str) -> PathBuf {
    let n = TEST_CTR.fetch_add(1, Ordering::SeqCst);
    std::env::temp_dir().join(format!("test_{}_{}.dat", name, n))
}

fn dummy_seed() -> Vec<u8> {
    // Derive a deterministic 64-byte seed from the test mnemonic
    use std::str::FromStr;
    let mnemonic = bip39::Mnemonic::from_str(TEST_MNEMONIC).unwrap();
    mnemonic.to_seed("").to_vec()
}

#[test]
fn test_save_load_roundtrip() {
    let seed = dummy_seed();
    let path = test_path("roundtrip");
    let password = "correct-horse-battery-staple";
    let mnemonic = Some(TEST_MNEMONIC);

    save_wallet(&path, &seed, mnemonic, password).unwrap();
    let (loaded_seed, loaded_mnemonic) = load_wallet(&path, password).unwrap();

    assert_eq!(loaded_seed, seed);
    assert_eq!(loaded_mnemonic.as_deref(), mnemonic);

    let _ = fs::remove_file(&path);
}

#[test]
fn test_wrong_password_fails() {
    let seed = dummy_seed();
    let path = test_path("wrong_pw");

    save_wallet(&path, &seed, None, "correct-password").unwrap();
    let result = load_wallet(&path, "wrong-password");

    assert!(matches!(result, Err(StorageError::DecryptionFailed)));

    let _ = fs::remove_file(&path);
}

#[test]
fn test_corrupted_ciphertext_fails() {
    let seed = dummy_seed();
    let path = test_path("corrupt");

    save_wallet(&path, &seed, None, "password").unwrap();

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
    let seed = dummy_seed();
    let path = test_path("format");

    save_wallet(&path, &seed, None, "password").unwrap();

    let data = fs::read(&path).unwrap();

    assert_eq!(data[0], 0x02, "version byte must be 0x02");
    assert!(
        data.len() > HEADER_LEN,
        "file must contain ciphertext after header"
    );
    // Salt and nonce should be non-zero (randomly generated)
    assert_ne!(&data[1..17], &[0u8; 16], "salt must not be zero");
    assert_ne!(&data[17..29], &[0u8; 12], "nonce must not be zero");

    let _ = fs::remove_file(&path);
}
